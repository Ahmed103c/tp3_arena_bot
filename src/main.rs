#![allow(dead_code)]

mod miner;
mod pow;
mod protocol;
mod state;
mod strategy;

#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::thread;
#[allow(unused_imports)]
use std::time::Duration;

use tungstenite::{connect, Message};
use uuid::Uuid;

use protocol::{ClientMsg, ServerMsg};

// ─── Configuration ──────────────────────────────────────────────────────────

const SERVER_URL: &str = "ws://127.0.0.1:4004/ws";
const TEAM_NAME: &str = "Team AhMed";
const AGENT_NAME: &str = "Super AhMed 🔥";
const NUM_MINERS: usize = 4;

fn main() {
    println!("[*] Connexion à {SERVER_URL}...");
    let (mut ws, _response) = connect(SERVER_URL).expect("impossible de se connecter au serveur");
    println!("[*] Connecté !");

    // ── Attendre le Hello ────────────────────────────────────────────────
    let agent_id: Uuid = match read_server_msg(&mut ws) {
        Some(ServerMsg::Hello { agent_id, tick_ms }) => {
            println!("[*] Hello reçu : agent_id={agent_id}, tick={tick_ms}ms");
            agent_id
        }
        other => panic!("premier message inattendu : {other:?}"),
    };

    // ── S'enregistrer ────────────────────────────────────────────────────
    send_client_msg(
        &mut ws,
        &ClientMsg::Register {
            team: TEAM_NAME.into(),
            name: AGENT_NAME.into(),
        },
    );
    println!("[*] Enregistré en tant que {AGENT_NAME} (équipe {TEAM_NAME})");

    // ── Partie 1 : État partagé ──────────────────────────────────────────
    let shared_state = state::new_shared_state(agent_id);

    // ── Partie 2 : Pool de mineurs ───────────────────────────────────────
    let miner_pool = miner::MinerPool::new(NUM_MINERS);

    // ── Partie 3 : Stratégie ─────────────────────────────────────────────
    // On utilise ValueWeightedStrategy par défaut : elle combine distance et valeur.
    // Pour basculer vers NearestResourceStrategy, remplacer la ligne ci-dessous.
    let strategy: Box<dyn strategy::Strategy> =
        Box::new(strategy::ValueWeightedStrategy::default());

    // ── Passer le socket en mode non-bloquant ────────────────────────────
    match ws.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(tcp) => {
            tcp.set_nonblocking(true).expect("set_nonblocking failed");
        }
        _ => {
            // Pour les streams TLS, le set_nonblocking se fait via le TcpStream sous-jacent.
            // En pratique, ws://... est toujours Plain.
            eprintln!("[!] Stream non-Plain : set_nonblocking ignoré");
        }
    }

    let mut last_move_tick: u64 = 0;
    let mut last_heartbeat_tick: u64 = 0;
    let mut pending_challenges: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let mut my_score: u32 = 0;
    let mut pending_nonces: std::collections::HashMap<Uuid, miner::MineResult> = 
        std::collections::HashMap::new();
    // ── Partie 4 : Boucle principale ─────────────────────────────────────
    loop {
        // 1. Lire les messages WS (non bloquant, vider le buffer)
        loop {
            match ws.read() {
                Ok(Message::Text(text)) => {
                    let msg: ServerMsg = match serde_json::from_str(&text) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("[!] Parse error: {e}");
                            continue;
                        }
                    };
                    // Mettre à jour l'état partagé
                    shared_state.lock().unwrap().update(&msg);

                    match msg {

                        ServerMsg::PowChallenge { tick, seed, resource_id, target_bits, .. } => {
                            // Éviter de soumettre deux fois le même challenge
                            if pending_challenges.insert(resource_id) {
                                println!("[~] Challenge reçu : resource={resource_id} bits={target_bits}");
                                miner_pool.submit(miner::MineRequest {
                                    seed,
                                    tick,
                                    resource_id,
                                    agent_id,
                                    target_bits,
                                });
                            }
                        }
                        
                        // Dans le handler PowResult :
                        ServerMsg::PowResult { resource_id, winner } => {
                            pending_challenges.remove(&resource_id);
                            miner_pool.cancel(resource_id);
                            if winner == agent_id {
                                my_score += 1;  // ← incrémenter localement
                                println!("[✓] Ressource minée ! Score local : {my_score}/{}", 
                                    shared_state.lock().unwrap().goal);
                            } else {
                                println!("[✗] Ressource {resource_id} gagnée par {winner}");
                            }
                        }

                        // Dans le handler Win :
                        ServerMsg::Win { team } => {
                            println!("[!] Partie terminée — victoire de {team} ! (score final: {my_score}/{})",
                                shared_state.lock().unwrap().goal);
                            return;
                        }

                            ServerMsg::Error { message } => {
                                eprintln!("[!] Erreur serveur : {message}");
                            }

                            _ => {}
                        }
                    }

                Ok(Message::Ping(data)) => {
                    // Répondre aux pings WebSocket
                    let _ = ws.send(Message::Pong(data));
                }

                Ok(_) => {} // ignorer les autres types de messages

                Err(tungstenite::Error::Io(ref e))
                    if e.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    break; // buffer vide, on sort de la boucle de lecture
                }

                Err(e) => {
                    eprintln!("[!] Erreur WS: {e}");
                    break;
                }
            }
        }

        // 2. Soumettre les nonces trouvés par le pool de mineurs
        // Au lieu de ignorer le nonce non-adjacent, le stocker
        while let Some(result) = miner_pool.try_recv() {
            let (pos, resource_pos) = {
                let st = shared_state.lock().unwrap();
                let rpos = st.resources.iter()
                    .find(|r| r.resource_id == result.resource_id)
                    .map(|r| (r.x, r.y));
                (st.position, rpos)
            };

            if let Some((rx, ry)) = resource_pos {
                let dist = (pos.0 as i32 - rx as i32).abs()
                        + (pos.1 as i32 - ry as i32).abs();
                if dist <= 1 {
                    // Adjacent → soumettre immédiatement
                    println!("[→] Soumission nonce={} resource={}", result.nonce, result.resource_id);
                    send_client_msg(&mut ws, &ClientMsg::PowSubmit {
                        tick: result.tick,
                        resource_id: result.resource_id,
                        nonce: result.nonce,
                    });
                    pending_challenges.remove(&result.resource_id);
                } else {
                    // Pas encore adjacent → garder en attente
                    println!("[⏳] Nonce prêt, en attente d'adjacence (dist={dist})");
                    pending_nonces.insert(result.resource_id, result);
                }
            }
        }

        // Vérifier les nonces en attente à chaque tick
        pending_nonces.retain(|resource_id, result| {
            let (pos, resource_pos) = {
                let st = shared_state.lock().unwrap();
                let rpos = st.resources.iter()
                    .find(|r| r.resource_id == *resource_id)
                    .map(|r| (r.x, r.y));
                (st.position, rpos)
            };

            match resource_pos {
                None => false, // ressource disparue → supprimer
                Some((rx, ry)) => {
                    let dist = (pos.0 as i32 - rx as i32).abs()
                            + (pos.1 as i32 - ry as i32).abs();
                    if dist <= 1 {
                        println!("[→] Soumission différée nonce={} resource={}", result.nonce, resource_id);
                        send_client_msg(&mut ws, &ClientMsg::PowSubmit {
                            tick: result.tick,
                            resource_id: *resource_id,
                            nonce: result.nonce,
                        });
                        pending_challenges.remove(resource_id);
                        false // supprimer après soumission
                    } else {
                        true // garder en attente
                    }
                }
            }
        });

        // 3. Stratégie de déplacement — un Move par tick (pas un par itération)
        let (current_tick, movement) = {
            let st = shared_state.lock().unwrap();
            let mv = strategy.next_move(&st);
            (st.tick, mv)
        };

        if current_tick > last_move_tick {
            if let Some((dx, dy)) = movement {
                send_client_msg(&mut ws, &ClientMsg::Move { dx, dy });
            }
            last_move_tick = current_tick;
        }

        // 4. Heartbeat toutes les 10 ticks
        if current_tick > last_heartbeat_tick + 10 {
            send_client_msg(&mut ws, &ClientMsg::Heartbeat { tick: current_tick });
            last_heartbeat_tick = current_tick;
        }

        // Pause courte pour ne pas saturer le CPU
        thread::sleep(Duration::from_millis(20));
    }
}

// ─── Fonctions utilitaires ───────────────────────────────────────────────────

type WsStream = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

fn read_server_msg(ws: &mut WsStream) -> Option<ServerMsg> {
    match ws.read() {
        Ok(Message::Text(text)) => serde_json::from_str(&text).ok(),
        Ok(_) => None,
        Err(e) => {
            eprintln!("[!] Erreur WS lecture : {e}");
            None
        }
    }
}

fn send_client_msg(ws: &mut WsStream, msg: &ClientMsg) {
    let json = serde_json::to_string(msg).expect("sérialisation échouée");
    ws.send(Message::Text(json.into())).expect("envoi WS échoué");
}