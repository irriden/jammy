apt install protobuf-compiler rustup git
./connect_nodes.sh 2> /dev/null
./fund.sh
git clone https://github.com/irriden/jammy.git
cd jammy
git clone --branch attackathon https://github.com/carlaKC/lnd.git
export LND_REPO_DIR=./lnd
cargo run
