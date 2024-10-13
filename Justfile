set windows-shell := ["powershell"]

@check *args:
    just check-all {{args}}
    just check-individual-settings {{args}}
    just check-async-runtimes {{args}}
    just check-wasm {{args}}

@check-wasm *args:
    just check-all --target wasm32-unknown-unknown {{args}}
    just check-individual-settings --target wasm32-unknown-unknown {{args}}

check-all *args:
    cargo clippy --all-features {{args}}

check-no-settings *args:
    cargo clippy {{args}} --no-default-features --features async-io

[linux]
[macos]
check-individual-settings *args:
    python3 scripts/check-individual-settings.py {{args}}

[windows]
check-individual-settings *args:
    python scripts/check-individual-settings.py {{args}}

[linux]
check-async-runtimes *args:
    cargo clippy --no-default-features --features=_all-preferences,tokio {{args}}
    cargo clippy --no-default-features --features=_all-preferences,async-io {{args}}

[windows]
[macos]
check-async-runtimes *args:
