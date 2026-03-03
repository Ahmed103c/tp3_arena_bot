# TP 3 — Bot d'Arène Multi-Thread

### Sujet

Dans ce TP, vous allez implémenter un **bot autonome** qui se connecte à un serveur d'arène via WebSocket, se déplace sur une carte en grille, et **mine des ressources en parallèle** grâce à un pool de threads.

Le serveur héberge une arène compétitive : plusieurs bots (potentiellement de différentes équipes) s'affrontent pour miner des ressources en résolvant des challenges de **Proof-of-Work** (trouver un nonce dont le hash blake3 commence par N bits à zéro). La première équipe à atteindre le score objectif gagne.

### Ce qui est fourni

| Fichier | Contenu |
|---------|---------|
| `src/main.rs` | Connexion WebSocket, enregistrement, squelette de la boucle principale |
| `src/protocol.rs` | Enums `ServerMsg` / `ClientMsg` avec serde (dé)sérialisation |
| `src/pow.rs` | Fonctions `pow_search()` et `pow_valid()` pour le Proof-of-Work |

### Ce que vous devez implémenter

| Fichier | Travail demandé |
|---------|----------------|
| `src/state.rs` | Définir `GameState` et le partager entre threads via `Arc<Mutex<>>` |
| `src/miner.rs` | Créer un pool de N threads mineurs communiquant par `mpsc::channel` |
| `src/strategy.rs` | Définir un trait `Strategy` et implémenter `NearestResourceStrategy` |
| `src/main.rs` | Câbler le tout dans la boucle principale (à compléter) |

---

## Protocole du serveur

### Connexion

1. Se connecter en WebSocket à `ws://<host>:4000/ws`
2. Le serveur envoie immédiatement un message `Hello`
3. Répondre avec `Register` pour rejoindre l'arène

### Messages serveur → client

#### `Hello`
```json
{"type": "Hello", "data": {"agent_id": "uuid", "tick_ms": 100}}
```
Premier message reçu. Contient votre identifiant unique et l'intervalle entre les ticks.

#### `State`
```json
{"type": "State", "data": {
  "tick": 42,
  "width": 64, "height": 48,
  "goal": 10,
  "obstacles": [[5, 3], [10, 7]],
  "resources": [["uuid", 12, 8, 72, 3]],
  "agents": [["uuid", "nom", "equipe", 3, 15, 20]]
}}
```
Envoyé à chaque tick. Contient l'état complet du jeu :
- `resources` : `[id, x, y, expires_at, value]`
- `agents` : `[id, name, team, score, x, y]`

#### `PowChallenge`
```json
{"type": "PowChallenge", "data": {
  "tick": 40, "seed": "hex...", "resource_id": "uuid",
  "x": 12, "y": 8, "target_bits": 20, "expires_at": 70, "value": 3
}}
```
Un nouveau challenge de minage est apparu. Trouver un `nonce` tel que :
```
blake3(seed ‖ tick_LE ‖ resource_id ‖ agent_id ‖ nonce_LE)
```
commence par `target_bits` bits à zéro. La fonction `pow_search()` dans `pow.rs` fait ce calcul.

**Contrainte** : vous devez être **adjacent** à la ressource (distance Manhattan = 1) pour soumettre une solution.

#### `PowResult`
```json
{"type": "PowResult", "data": {"resource_id": "uuid", "winner": "uuid"}}
```
Un agent a résolu le challenge. La ressource disparaît.

#### `Win`
```json
{"type": "Win", "data": {"team": "equipe"}}
```
Une équipe a atteint le score objectif. La partie est terminée.

#### `Mining`
```json
{"type": "Mining", "data": {"agent_id": "uuid", "resource_id": "uuid", "on": true}}
```
Notification qu'un agent commence ou arrête de miner.

#### `Error`
```json
{"type": "Error", "data": {"message": "..."}}
```

### Messages client → serveur

#### `Register`
```json
{"type": "Register", "data": {"team": "mon_equipe", "name": "bot_1"}}
```

#### `Move`
```json
{"type": "Move", "data": {"dx": 1, "dy": 0}}
```
Se déplacer d'une case. `dx` et `dy` doivent être dans `{-1, 0, 1}`. Le mouvement est bloqué si la case cible contient un obstacle, une ressource, ou un autre agent.

#### `PowSubmit`
```json
{"type": "PowSubmit", "data": {"tick": 40, "resource_id": "uuid", "nonce": 123456}}
```
Soumettre une solution de minage. Le `tick` doit correspondre au tick du challenge.

#### `Mining`
```json
{"type": "Mining", "data": {"resource_id": "uuid", "on": true}}
```
Signaler que vous commencez/arrêtez de miner (informatif).

#### `Heartbeat`
```json
{"type": "Heartbeat", "data": {"tick": 42}}
```

---

## Guide d'implémentation

### Partie 1 — État partagé (`state.rs`)

Définir une structure `GameState` qui contient toutes les informations nécessaires au bot (position, ressources, agents, scores...).

L'encapsuler dans `Arc<Mutex<GameState>>` (alias `SharedState`) pour la partager entre :
- Le **thread lecteur WS** qui la met à jour
- Le **thread principal** qui la consulte pour décider des actions

```rust
// Pattern à utiliser :
let state = Arc::new(Mutex::new(GameState::new(agent_id)));

// Dans un thread :
let state_clone = Arc::clone(&state);
thread::spawn(move || {
    let mut guard = state_clone.lock().unwrap();
    guard.update(&msg);
});
```

### Partie 2 — Pool de mineurs (`miner.rs`)

Créer N threads qui attendent des challenges et cherchent des nonces en parallèle.

Communication par **2 channels `mpsc`** :
- `Sender<MineRequest>` : le thread principal envoie les challenges
- `Receiver<MineResult>` : le thread principal récupère les solutions

Le `Receiver<MineRequest>` est partagé entre les N threads via `Arc<Mutex<Receiver<>>>` : chaque thread fait `lock() → recv()` en boucle.

```
Principal ──MineRequest──> [Arc<Mutex<Receiver>>] ──> Thread 0
                                                  ──> Thread 1
                                                  ──> Thread 2
                                                  ──> Thread 3

Thread * ──MineResult──> Principal
```

### Partie 3 — Stratégie (`strategy.rs`)

Définir un trait `Strategy` avec une méthode `next_move(&self, state: &GameState) -> Option<(i8, i8)>`.

Implémenter `NearestResourceStrategy` qui calcule la direction vers la ressource la plus proche.

Utiliser `Box<dyn Strategy>` dans `main.rs` pour pouvoir changer de stratégie sans modifier le reste du code.

### Partie 4 — Assemblage (`main.rs`)

Compléter la boucle principale :
1. Lancer un thread lecteur WS qui met à jour le `SharedState` et forward les messages importants via un channel `mpsc`
2. Quand un `PowChallenge` arrive → l'envoyer au `MinerPool`
3. Quand le `MinerPool` trouve un nonce → envoyer `PowSubmit` au serveur
4. À chaque itération → consulter la `Strategy` et envoyer `Move`

---

## Critères de validation

- `cargo check` passe sans erreur
- `cargo clippy` sans avertissement
- Le bot se connecte, s'enregistre, et apparaît sur la carte
- Le bot se déplace vers les ressources (visible sur la visu du serveur)
- Le bot mine avec succès au moins 1 ressource
- Le code utilise : `Arc<Mutex<>>`, `mpsc::channel`, `thread::spawn`, `Box<dyn Strategy>`

---

## Bonus

- Implémenter d'autres stratégies : `RandomStrategy`, `FleeStrategy`, `HybridStrategy`
- Optimiser le minage : annuler les mineurs quand un `PowResult` arrive pour une ressource déjà en cours de minage
- Gérer plusieurs challenges simultanément avec priorité par distance
- Ajouter du logging pour débugger le comportement du bot
