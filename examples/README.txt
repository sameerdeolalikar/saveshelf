Use fictional data to try SaveShelf without touching real game saves:

mkdir demo-save
printf 'level=3\ncoins=42\n' > demo-save/slot1.sav
cargo run -- snapshot demo-save --vault demo-vault
cargo run -- list --vault demo-vault

Copy the snapshot path printed above into:
cargo run -- verify SNAPSHOT_PATH
cargo run -- restore SNAPSHOT_PATH --output recovered

Recovered bytes are in recovered/files. Leave any real save folder untouched
until you have verified the result. Do not commit personal save files.
