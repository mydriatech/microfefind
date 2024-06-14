#!/bin/sh

#   Copyright 2024 MydriaTech AB
#
#   Licensed under the Apache License 2.0 with Free world makers exception
#   1.0.0 (the "License"); you may not use this file except in compliance with
#   the License. You should have obtained a copy of the License with the source
#   or binary distribution in file named
#
#       LICENSE-Apache-2.0-with-FWM-Exception-1.0.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"

srcDirName="$(ls -d "${CARGO_HOME}"/registry/src/index.crates.io-*)"

targetDirName="./licenses"
rm -rf "$targetDirName"
mkdir -p "$targetDirName"

cargo tree --target x86_64-unknown-linux-musl --prefix none --edges no-build --no-dedupe | \
    sort | \
    uniq | \
    grep -v 'aol_' | \
    sed '/^[[:space:]]*$/d' | \
    sed 's| (proc-macro)$||' | \
    sed 's| v|-|' | \
    while read -r line ; do
        mkdir "$targetDirName/$line"
        cp "$srcDirName/$line"/LICENSE* "$targetDirName/$line/"
        cp "$srcDirName/$line"/COPYRIGHT* "$targetDirName/$line/" 2>/dev/null
        cp "$srcDirName/$line"/COPYING* "$targetDirName/$line/" 2>/dev/null
    done

rustUpToolchainDirName="$(ls -d "$RUSTUP_HOME"/toolchains/*)"
rustDocDirName="$rustUpToolchainDirName/share/doc/rust"
rustVersion="$(rustc --version | sed 's|rustc \(.*\) (.*|\1|g')"
mkdir "$targetDirName/rust-$rustVersion"
cp "$rustDocDirName"/LICENSE* "$targetDirName/rust-$rustVersion/"
cp "$rustDocDirName"/COPYRIGHT* "$targetDirName/rust-$rustVersion/"

echo "Handling special cases..."
depAllocStdlibDirName="$(ls -d licenses/alloc-stdlib*)"
curl --silent \
    -L https://raw.githubusercontent.com/dropbox/rust-alloc-no-stdlib/master/LICENSE \
    -o "$depAllocStdlibDirName/LICENSE"
depConvertCaseDirName="$(ls -d licenses/convert_case*)"
curl --silent \
    -L https://raw.githubusercontent.com/rutrum/convert-case/master/LICENSE \
    -o "$depConvertCaseDirName/LICENSE"

depHttpBodyDirName="$(ls -d licenses/http_body*)"
curl --silent \
    -L https://raw.githubusercontent.com/hyperium/http-body/master/LICENSE \
    -o "$depHttpBodyDirName/LICENSE"

depKubeRsDirName="$(ls -d licenses/kube-0*)"
curl --silent \
    -L https://raw.githubusercontent.com/kube-rs/kube/main/LICENSE \
    -o "$depKubeRsDirName/LICENSE"
depDirName="$(ls -d licenses/kube-client-*)"
cp "$depKubeRsDirName/LICENSE" "$depDirName/LICENSE"
depDirName="$(ls -d licenses/kube-core-*)"
cp "$depKubeRsDirName/LICENSE" "$depDirName/LICENSE"
depDirName="$(ls -d licenses/kube-runtime-*)"
cp "$depKubeRsDirName/LICENSE" "$depDirName/LICENSE"

depMuslLibcDirName="licenses/musl-libc"
mkdir "$depMuslLibcDirName"
curl --silent \
    -L https://git.musl-libc.org/cgit/musl/plain/COPYRIGHT \
    -o "$depMuslLibcDirName/LICENSE"

cp LICENSE* licenses/
cp COPYRIGHT* licenses/ 2>/dev/null

du -hs licenses/
