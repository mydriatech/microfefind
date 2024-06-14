# Contributing to the project

_This project is not yet ready to receive contributions._

You will be expected to sign off on a [Developer Certificate of Origin](https://developercertificate.org/) and transfer your copy-right to the Licensor to enable use of your contributions under a separate commercial license as well.


## Development

### Building the binary for Alpine Linux on Debian/Ubuntu

```
sudo apt-get update
sudo apt-get install musl-tools -y
rustup target add x86_64-unknown-linux-musl
cargo build --target=x86_64-unknown-linux-musl
```

