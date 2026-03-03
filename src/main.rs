// Le squelette contient du code fourni pas encore utilisé — c'est normal.
#![allow(dead_code)]

mod miner;
mod pow;
mod protocol;
mod state;
mod strategy;

// Ces imports seront utilisés dans votre implémentation.
#[allow(unused_imports)]
use std::sync::{Arc, Mutex};
#[allow(unused_imports)]
use std::thread;
#[allow(unused_imports)]
use std::time::Duration;

use tungstenite::{connect, Message};
#[allow(unused_imports)]
use uuid::Uuid;

use protocol::{ClientMsg, ServerMsg};

// ─── Configuration ──────────────────────────────────────────────────────────

const SERVER_URL: &str = "ws://127.0.0.1:4004/ws";
const TEAM_NAME: &str = "mon_equipe";
const AGENT_NAME: &str = "bot_1";
const NUM_MINERS: usize = 4;

fn main() {
    println!("[*] Connexion à {SERVER_URL}...");
    let (mut ws, _response) = connect(SERVER_URL).expect("impossible de se connecter au serveur");
    println!("[*] Connecté !");

    // ── Attendre le Hello ────────────────────────────────────────────────
    #[allow(unused_variables)] // Vous utiliserez agent_id dans votre implémentation.
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

    // ─────────────────────────────────────────────────────────────────────
    //  À PARTIR D'ICI, C'EST À VOUS DE JOUER !
    //
    //  Objectif : implémenter la boucle principale du bot.
    //
    //  Étapes suggérées :
    //
    //  1. Créer l'état partagé (state::SharedState)
    //     let shared_state = state::SharedState::new(agent_id);
    //
    //  2. Créer le pool de mineurs (miner::MinerPool)
    //     let miner_pool = miner::MinerPool::new(NUM_MINERS);
    //
    //  3. Créer la stratégie de déplacement
    //     let strategy: Box<dyn strategy::Strategy> = Box::new(strategy::NearestResourceStrategy);
    //
    //  4. Séparer la WebSocket en lecture/écriture
    //     Utiliser ws.into_inner() pour récupérer le TcpStream puis séparer
    //     via std::io::Read/Write. Sinon, approche plus simple ci-dessous :
    //
    //  ─── Approche simplifiée (recommandée) ─────────────────────────────
    //
    //  Utiliser ws.read() dans un thread dédié qui :
    //    a) parse les ServerMsg
    //    b) met à jour le SharedState
    //    c) envoie les PowChallenge au MinerPool via un channel
    //
    //  Le thread principal :
    //    a) vérifie si le MinerPool a trouvé une solution → envoie PowSubmit
    //    b) consulte la stratégie pour décider du prochain mouvement → envoie Move
    //    c) dort un court instant (ex: 50ms) pour ne pas surcharger
    //
    //  Contrainte : la WebSocket (tungstenite) n'est pas Send si on utilise
    //  la version par défaut. Vous devrez garder toutes les écritures WS
    //  dans le thread principal, et utiliser des channels pour communiquer
    //  depuis le thread lecteur.
    // ─────────────────────────────────────────────────────────────────────

    // TODO: Partie 1 — Créer le SharedState (voir state.rs)

    // TODO: Partie 2 — Créer le MinerPool (voir miner.rs)

    // TODO: Partie 3 — Créer la stratégie (voir strategy.rs)

    // TODO: Partie 4 — Lancer le thread lecteur WS
    //
    // Indice : il faut un channel pour recevoir les messages du thread lecteur
    // car la WebSocket ne peut pas être partagée entre threads.
    //
    // let (tx, rx) = std::sync::mpsc::channel::<ServerMsg>();
    //
    // Le thread lecteur lit les messages, met à jour le state, et forward
    // les messages importants via le channel.

    // TODO: Partie 5 — Boucle principale
    //
    // loop {
    //     // 1. Lire les messages du thread lecteur (rx.try_recv())
    //     //    - PowChallenge → envoyer au MinerPool
    //     //    - Win → afficher et quitter
    //     //    - Autres → déjà traités par le thread lecteur
    //
    //     // 2. Vérifier si le MinerPool a trouvé un nonce
    //     //    → envoyer ClientMsg::PowSubmit
    //
    //     // 3. Consulter la stratégie pour le prochain mouvement
    //     //    → envoyer ClientMsg::Move
    //
    //     // 4. Dormir un peu
    //     thread::sleep(Duration::from_millis(50));
    // }

    println!("[!] TODO: implémenter la boucle principale");
}

// ─── Fonctions utilitaires (fournies) ───────────────────────────────────────

type WsStream = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

/// Lit un message du serveur et le désérialise.
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

/// Sérialise et envoie un message au serveur.
fn send_client_msg(ws: &mut WsStream, msg: &ClientMsg) {
    let json = serde_json::to_string(msg).expect("sérialisation échouée");
    ws.send(Message::Text(json.into())).expect("envoi WS échoué");
}
