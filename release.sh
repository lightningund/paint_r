mkdir binaries/$1
cargo build -r
cp target/release/paintr binaries/$1/paintr_linux_$1
cargo build -r --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/paintr.exe binaries/$1/paintr_windows_$1.exe