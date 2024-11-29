### DEFAULT ###

# Install dependencies for the default feature on x86_64-unknown-linux-musl
install-default-x86_64-unknown-linux-musl:
    sudo apt-get update
    sudo apt-get install -y musl-tools

# Install dependencies for the default feature on aarch64-unknown-linux-musl
install-default-aarch64-unknown-linux-musl:
    sudo apt-get update
    sudo apt-get install -y musl-tools

# Install dependencies for the default feature on i686-unknown-linux-gnu
install-default-i686-unknown-linux-gnu:
    sudo apt-get update
    sudo apt-get install -y gcc-multilib-i686-linux-gnu gcc-i686-linux-gnu
