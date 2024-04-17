apt install protobuf-compiler cargo git
./connect_nodes.sh 2> /dev/null
./fund.sh
git clone https://github.com/irriden/jammy.git
cd jammy
git clone --branch attackathon https://github.com/carlaKC/lnd.git
export LND_REPO_DIR=/jammy/lnd
cargo run
