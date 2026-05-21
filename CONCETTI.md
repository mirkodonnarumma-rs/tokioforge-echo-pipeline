# Tokio Lab — Guida ai Concetti

Questo documento spiega **perché** il codice è strutturato come lo è, non solo **cosa** fa.

---

## 1. Il runtime: chi esegue il codice async?

Una `async fn` in Rust non fa nulla da sola. Restituisce un `Future` — un valore che
descrive una computazione da eseguire. Qualcuno deve fare il **polling** di quel Future,
cioè eseguirlo a piccoli passi fino al completamento.

Quel qualcuno è il **runtime Tokio**.

La macro `#[tokio::main]` trasforma questo:

```rust
#[tokio::main]
async fn main() {
    println!("ciao");
}
```

in qualcosa di equivalente a:

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            println!("ciao");
        });
}
```

`block_on` dice al thread principale: "blocca qui finché questo Future non è completato".
Il runtime crea un pool di thread OS in background (`multi_thread`) e distribuisce i
task su di essi.

---

## 2. Future, `.await`, e il modello di esecuzione

Un `Future` è uno **state machine** generato dal compilatore. Ogni punto `.await` è uno
stato possibile. Quando un Future deve aspettare (es: dati di rete non ancora arrivati),
restituisce `Poll::Pending` e il runtime può eseguire un altro Future.

```
task A: legge TCP → nessun dato → Poll::Pending
task B: legge TCP → ha dati    → Poll::Ready(n) → esegue
task A: il SO segnala dati    → poll di nuovo   → Poll::Ready(n) → esegue
```

Nessun thread bloccato ad aspettare. Questo è il vantaggio dell'I/O asincrono.

---

## 3. `tokio::spawn` — perché `Send + 'static`?

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
```

Due vincoli:

- **`Send`**: il Future può essere spostato da un thread worker a un altro. Tutti i dati
  che cattura devono essere `Send`. Un `Rc<T>` non è `Send`; un `Arc<T>` sì.

- **`'static`**: il Future non può contenere riferimenti a variabili che vivono sullo
  stack del chiamante. Se il chiamante tornasse prima che il task finisca, quei
  riferimenti punterebbero a memoria invalida. La soluzione è `async move {}`:
  si trasferisce la ownership dei dati dentro il Future.

```rust
let socket: TcpStream = ...;
// SBAGLIATO: &socket non è 'static
tokio::spawn(async { let _ = socket.read(&mut buf).await; });

// CORRETTO: socket è moved dentro il Future
tokio::spawn(async move { let _ = socket.read(&mut buf).await; });
```

---

## 4. L'echo server: architettura

```
main()
  │
  ├─ TcpListener::bind("127.0.0.1:8080")
  │
  ├─ watch::channel(false)   ← shutdown_tx / shutdown_rx
  │
  ├─ tokio::spawn(run_echo_server(listener, shutdown_rx))
  │
  ├─ signal::ctrl_c().await  ← blocca qui finché Ctrl+C
  │
  └─ shutdown_tx.send(true)  ← tutti i receiver si svegliano

run_echo_server(listener, shutdown_rx)
  │
  └─ loop:
      ├─ accept_or_shutdown()  ← select! tra accept() e shutdown
      │     ├─ nuova connessione → tokio::spawn(handle_client)
      │     └─ shutdown        → break
      │
      └─ join_set.join_next()  ← aspetta che tutti i task client finiscano

handle_client(socket, shutdown_rx)
  │
  └─ loop:
      └─ select!:
          ├─ socket.read() → echo
          └─ shutdown_rx.changed() → return (socket droppato → EOF al client)
```

### Perché `accept_or_shutdown` è una funzione separata?

Il problema del borrow checker: `shutdown_rx.changed()` richiede `&mut shutdown_rx`.
Se mettiamo tutto in un unico `select!`, nel ramo dell'accept vorremmo anche clonare
`shutdown_rx` — ma è ancora "in prestito" dalla espressione `changed()`.

Soluzione: isolare il `select!` in una funzione che prende `&mut shutdown_rx`,
la usa, e restituisce. Quando la funzione ritorna, il borrow è rilasciato.
Il chiamante può poi clonare `shutdown_rx` liberamente.

```rust
async fn accept_or_shutdown(
    listener: &TcpListener,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> Option<(TcpStream, SocketAddr)> {
    tokio::select! {
        result = listener.accept() => result.ok(),
        _ = shutdown_rx.changed() => None,
    }
}
```

---

## 5. `tokio::select!` — multiplexer di Future

`select!` aspetta il **primo** tra N Future che si completa, esegue il suo ramo,
e **droppa** tutti gli altri Future.

```rust
tokio::select! {
    result = listener.accept() => { /* gestisci */ }
    _ = signal::ctrl_c()       => { /* shutdown */ }
}
```

Nota importante: i Future non selezionati vengono **droppati**, non "messi in pausa
e ripresi". Ogni iterazione del loop ricrea i Future. Per `listener.accept()` va bene:
`accept()` è stateless. Ma per operazioni stateful (es: leggere dati parziali da un
buffer) bisogna fare attenzione.

---

## 6. Il canale `watch` — stato condiviso

`tokio::sync::watch` mantiene **un solo valore** osservabile da N receiver:

```
shutdown_tx.send(true)
    │
    ├─ run_echo_server: shutdown_rx.changed() si sveglia → break
    ├─ handle_client A: shutdown_rx.changed() si sveglia → return
    └─ handle_client B: shutdown_rx.changed() si sveglia → return
```

Perché `watch` e non `broadcast`?

| Caratteristica | `watch` | `broadcast` |
|---|---|---|
| Cosa conserva | solo l'ultimo valore | tutti i messaggi (fino a overflow) |
| Caso d'uso | stato (sì/no, config) | eventi (log, notifiche) |
| Messaggi persi? | sì (se overwritten) | sì (se il consumer è lento) |

Per lo shutdown ci interessa lo **stato attuale** ("è in shutdown?"), non la
sequenza di messaggi. `watch` è la scelta corretta.

### `changed()` e il meccanismo di versioning

Ogni receiver tiene traccia della "versione" dell'ultimo valore che ha osservato.
`changed()` aspetta che il valore raggiunga una versione più alta.

Quando cloniamo un receiver, il clone eredita la stessa versione del genitore.
Quindi:
- Se cloniamo il receiver **prima** di inviare `true`, il clone vedrà il cambiamento.
- Se cloniamo **dopo** che `true` è già stato inviato e osservato, il clone NON lo vede.

Nel nostro server, cloniamo sempre **prima** che il main loop veda lo shutdown
(perché la clone avviene nel ramo `accept`, che precede il break sul `changed`).

---

## 7. `JoinSet` — aspettare N task

```rust
let mut join_set: JoinSet<()> = JoinSet::new();
join_set.spawn(handle_client(socket, rx));
// ...
while join_set.join_next().await.is_some() {}
```

`JoinSet` raccoglie i JoinHandle dei task spawnati. `join_next()` aspetta il primo
che completa, rimuovendolo dal set. Quando il set è vuoto, il loop termina.

Questo garantisce lo **graceful shutdown**: quando `run_echo_server` riceve il segnale,
smette di accettare nuove connessioni e aspetta che tutti i client attivi completino
la loro operazione corrente.

---

## 8. RAII e TcpStream

Quando `handle_client` restituisce (per shutdown, EOF, o errore), il `TcpStream` viene
**droppato automaticamente**. Rust non richiede `socket.close()` esplicito.

Il Drop di `TcpStream` invia un `FIN` TCP al client. Il client vede EOF sulla sua read
(ritorna 0 byte). Questo è il meccanismo che fa sì che il client `nc` si disconnetta
quando il server fa shutdown.

---

## 9. Producer–Consumer con `mpsc`

`tokio::sync::mpsc::channel(capacity)` crea un canale bounded:

```
producer ──send──► [  buf  ] ──recv──► consumer
capacity = 4:      [ □□□□ ]
```

- Se il buffer è pieno, `send().await` **si sospende** (non blocca il thread!).
- Quando il consumer consuma un elemento, il producer si sveglia.
- Questo si chiama **backpressure**: la pressione risale dalla coda al produttore.

```
[P] Invio: msg-00   ← buffer: [00]
[P] Invio: msg-01   ← buffer: [00,01]
[P] Invio: msg-02   ← buffer: [00,01,02]
[P] Invio: msg-03   ← buffer: [00,01,02,03] — PIENO
           ↑ producer si sospende qui
[C] Ricevuto: msg-00 ← buffer: [01,02,03]
[P] Invio: msg-04   ← buffer: [01,02,03,04] — producer si sveglia
```

Quando il sender (`tx`) viene droppato, `rx.recv()` restituisce `None` — segnale
"il produttore ha finito". Il consumer può uscire dal suo loop.

---

## 10. Broadcast — fan-out

`tokio::sync::broadcast::channel(capacity)` invia **ogni messaggio a tutti i receiver**:

```
producer ──send──► ┌───────────┐
                   │ ring buf  │──subscribe──► consumer A (veloce)
                   │  size=8   │──subscribe──► consumer B (medio)
                   └───────────┘──subscribe──► consumer C (lento)
```

Differenze chiave rispetto a `mpsc`:

| | `mpsc` | `broadcast` |
|---|---|---|
| Consumer | uno solo | N (via `subscribe()`) |
| `send()` è async? | sì — blocca se pieno | **no** — mai blocca |
| Se il buffer è pieno? | producer aspetta | messaggi vecchi sovrascritti |
| Consumer lento riceve? | tutto (backpressure) | `Err(Lagged(n))` |

`broadcast` è adatto per **notifiche** dove perdere qualche evento è accettabile
(log, metriche, eventi UI). `mpsc` è per **pipeline di lavoro** dove ogni messaggio
deve essere elaborato esattamente una volta.

---

## 11. Test asincroni

### `#[tokio::test]`

```rust
#[tokio::test]
async fn test_echo_basic() {
    // ...
}
```

Equivalente a `#[test]` ma crea un runtime Tokio per il test. Senza questo,
non si può usare `.await` in un test.

### Porta 0

```rust
TcpListener::bind("127.0.0.1:0").await.unwrap()
```

La porta 0 dice all'OS: "assegnami una porta libera". I test paralleli non si
"pestano i piedi" perché ognuno ottiene una porta diversa.

### `tokio::time::timeout`

```rust
timeout(Duration::from_secs(2), client.read_exact(&mut buf))
    .await
    .unwrap()  // Err(Elapsed) se scade il timeout
    .unwrap()  // Err(io::Error) se la read fallisce
```

Senza timeout, un test che non riceve risposta rimane appeso per sempre.
`timeout` avvolge qualsiasi Future e restituisce `Err(Elapsed)` se non completa
entro il tempo indicato.

---

## 12. Struttura del progetto

```
tokio-lab/
├── Cargo.toml
├── src/
│   ├── lib.rs                    ← logica del server (run_echo_server)
│   └── bin/
│       ├── echo_server.rs        ← binario: main, log, ctrl_c
│       ├── prodcons_bounded.rs   ← demo mpsc + unit test
│       └── prodcons_broadcast.rs ← demo broadcast
└── tests/
    └── integration.rs            ← test di integrazione del server
```

`src/lib.rs` espone `run_echo_server` — usato sia dal binario che dai test di
integrazione. Questo evita la duplicazione e permette di testare la logica del server
senza avviare un processo separato.

---

## Comandi rapidi

```bash
# Server echo (Ctrl+C per fermare)
cargo run --bin echo_server

# Demo backpressure
cargo run --bin prodcons_bounded

# Demo broadcast con messaggi persi
cargo run --bin prodcons_broadcast

# Test completi
cargo test

# Test con output visibile
cargo test -- --nocapture

# Qualità
cargo clippy -- -D warnings
cargo fmt -- --check
```
