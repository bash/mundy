set windows-shell := ["powershell"]
python-bin := if os() == "windows" {
    "python"
} else {
    "python3"
}

@check *args:
    just check-all {{args}}
    just check-individual-settings {{args}}
    just check-async-runtimes {{args}}
    just check-wasm {{args}}
    just check-android {{args}}
    cargo test {{args}}

@check-wasm *args:
    just check-all --target wasm32-unknown-unknown {{args}}
    just check-individual-settings --target wasm32-unknown-unknown {{args}}

@check-android *args:
    just check-all --target aarch64-linux-android {{args}}
    just check-individual-settings --target aarch64-linux-android {{args}}

check-all *args:
    cargo clippy --all-features {{args}}

check-no-settings *args:
    cargo clippy {{args}} --no-default-features --features async-io

check-individual-settings *args:
    {{python-bin}} scripts/check-individual-settings.py {{args}}

[linux]
check-async-runtimes *args:
    cargo clippy --no-default-features --features=_all-preferences,tokio {{args}}
    cargo clippy --no-default-features --features=_all-preferences,async-io {{args}}

[windows]
[macos]
check-async-runtimes *args:
