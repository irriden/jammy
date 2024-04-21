apt install -y protobuf-compiler rustup git
./connect_nodes.sh 2> /dev/null
./fund.sh
git clone https://github.com/irriden/jammy.git --branch morning
cd jammy
git clone --branch attackathon https://github.com/carlaKC/lnd.git
export LND_REPO_DIR=/jammy/lnd
rustup default stable
cargo run
