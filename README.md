![OGit](assets/tokio-lab.png)
# tokioforge-echo-pipeline — TCP Echo Server & Producer–Consumer (Rust + Tokio)

Progetto didattico Rust che esplora il runtime async Tokio attraverso un dominio reale:
networking TCP e pattern di concorrenza. Niente strutture artificiali — solo codice che fa cose vere.

**Concetti trattati:** runtime async, task spawning, ownership cross-task, graceful shutdown,
`JoinSet`, canali bounded vs broadcast, testing asincrono con porta dinamica.

## Struttura

```
src/
  lib.rs                     # run_echo_server() — logica server riusabile
  bin/
    echo_server.rs           # binario: bind :8080, Ctrl+C graceful shutdown
    prodcons_bounded.rs      # demo mpsc::channel con backpressure (capacity=4)
    prodcons_broadcast.rs    # demo broadcast con 3 consumer a velocità diverse
tests/
  integration.rs             # test integrazione: porta 0, multi-client, shutdown
CONCETTI.md                  # guida ai concetti Tokio (perché, non solo cosa)
```

## Requisiti

- Rust stable ≥ 1.80 (edition 2024)
- Tokio 1.51 con feature `full`

## Eseguire

```bash
# Echo server su :8080
cargo run --bin echo_server

# Demo producer-consumer bounded (mpsc)
cargo run --bin prodcons_bounded

# Demo broadcast multi-consumer
cargo run --bin prodcons_broadcast

# Test
cargo test
```

## Roadmap

- [x] STEP 1 — scaffolding + echo server single-client
- [x] STEP 2 — concorrenza multi-client (`tokio::spawn`)
- [x] STEP 3 — graceful shutdown (`tokio::select!`, `watch` channel)
- [x] STEP 4 — refactoring in `lib.rs` + `JoinSet` per shutdown pulito
- [x] STEP 5 — Producer–Consumer bounded (`mpsc`, backpressure)
- [x] STEP 6 — variante broadcast multi-consumer (`broadcast`, lagged handling)
- [x] STEP 7 — test integrazione con porta 0 (no conflitti in parallelo)
- [x] STEP 8 — guida concetti async (`CONCETTI.md`)
- [x] STEP 9 — logging strutturato (`tracing` + `tracing-subscriber`, `RUST_LOG`)
- [x] STEP 10 — gestione errori robusta: log EOF/errori con addr, warn su write fail
- [x] STEP 11 — script demo (`demo.sh`): build + test suite + smoke test echo + demo binary

## License

Licensed under [MIT license](LICENSE).
