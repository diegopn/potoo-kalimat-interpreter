<p align="center">
  <img src="https://diegopn.github.io/potoo-site/src/assets/images/language-interpreter-icon.svg?v=1c651ca7" alt="Potoo IDE language interpreter icon" width="120" />
</p>

<h1 align="center">Kalimat Interpreter</h1>

<p align="center">A host-driven Rust interpreter for the documented Kalimat runtime.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-kalimat-interpreter/releases">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/kalimat/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/kalimat/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img alt="Cargo" src="https://img.shields.io/badge/Cargo-DEA584?style=for-the-badge&logo=rust&logoColor=white"></a>
  <img alt="Rust library" src="https://img.shields.io/badge/Rust_library-4A86CF?style=for-the-badge">
  <img alt="CLI" src="https://img.shields.io/badge/CLI-6A4C93?style=for-the-badge">
</p>

This Rust package is part of the Potoo IDE ecosystem and is separate from the desktop interface. Compatibility with the IDE has not been implemented or verified.

## Reference baseline

The implementation targets the Arabic syntax and runtime features shown in the official distribution's bundled reference and examples guide PDFs. Those documents are legacy references: the deck shows dates from 2010–2011 and does not define every detail of the current distribution. Where the references are silent, this crate avoids claiming compatibility.

The documented baseline covers dynamic values, input and output, conditions and loops, arrays and dictionaries, procedures and functions, built-ins, tail calls, drawing and sprites, objects and methods, events, cooperative processes and CSP channels, files, and units. See [reference coverage](REFERENCE-COVERAGE.md) for implemented behavior and host responsibilities. This crate does not reproduce the Kalimat IDE.

## Current limits

The current official site advertises calling C procedures through FFI, but the bundled language PDFs do not specify its syntax, ABI type mapping, ownership rules, or error behavior. This crate does not implement native symbol loading or claim FFI compatibility; those contracts cannot be derived from the available language references.

The PDFs explicitly say that `do/while`, `break`, and `continue` are unavailable, custom data types are not yet supported as dictionary keys, constructors are absent, and process launch is asynchronous without true parallel execution. These limits are preserved in the documented profile.

## Status

This project remains under evaluation. Compatibility with the current Arabic distribution has not been established.

## Library use

Call `parse(source)`, then run the resulting `Program` with `Interpreter::new(host).run(&program)`. `Host` supplies random values and routes graphics, sprites, GUI calls, event registration, channels, file operations, and asynchronous launch. Its default input and output methods use the terminal.

## Command line

Save a minimal program in `example.kalimat`:

```text
اطبع 42...
```

Run it from this repository or after installing the command:

```sh
cargo run --locked -- example.kalimat
potoo-kalimat example.kalimat
```

The runner reads the UTF-8 source file path from its command-line argument and resolves unit files beside the source file. It provides terminal input/output, random values, and delays. Graphical, file, event, and concurrent host operations need an application-specific host.

### Build and install from source

Install Rust 1.93 or later, then build and install the CLI from this repository:

```sh
cargo build --release --locked --bin potoo-kalimat
cargo install --path . --locked --bin potoo-kalimat
```

Cargo installs the command in its configured binary directory, usually `~/.cargo/bin`. Ensure that directory is on `PATH`, then run `potoo-kalimat example.kalimat`.

### Install a prebuilt release

After a release is published, download `SHA256SUMS` and the archive for your platform from the [GitHub Releases page](https://github.com/diegopn/potoo-kalimat-interpreter/releases). The archive names are:

- Linux x86_64 GNU: `potoo-kalimat-interpreter-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- Windows x86_64 MSVC: `potoo-kalimat-interpreter-<tag>-x86_64-pc-windows-msvc.zip`
- Intel macOS: `potoo-kalimat-interpreter-<tag>-x86_64-apple-darwin.tar.gz`
- Apple silicon macOS: `potoo-kalimat-interpreter-<tag>-aarch64-apple-darwin.tar.gz`

Replace `<tag>` with the downloaded release tag and use the filename for your platform. The Linux build is made on Ubuntu 22.04 and may require a sufficiently recent glibc. On Linux or macOS, set `archive` to the exact downloaded filename and verify its matching checksum entry before extracting:

```sh
set -euo pipefail
archive="potoo-kalimat-interpreter-<tag>-x86_64-unknown-linux-gnu.tar.gz"
matches="$(awk -v archive="$archive" '$2 == archive { count++; line = $0 } END { if (count != 1) exit 1; print line }' SHA256SUMS)"
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s\n' "$matches" | sha256sum --check -
else
  printf '%s\n' "$matches" | shasum -a 256 --check -
fi
mkdir -p "$HOME/.local/bin"
tar -xzf "$archive"
install -m 0755 "${archive%.tar.gz}/potoo-kalimat" "$HOME/.local/bin/potoo-kalimat"
export PATH="$HOME/.local/bin:$PATH"
```

For Intel or Apple silicon macOS, use the corresponding archive name above. Add the `PATH` line to `~/.profile` or your shell configuration to keep it for future sessions. On Windows PowerShell, use the Windows archive and verify the exact entry before extraction:

```powershell
$archive = "potoo-kalimat-interpreter-<tag>-x86_64-pc-windows-msvc.zip"
$checksumEntries = @(Get-Content .\SHA256SUMS | Where-Object { $_ -match "^\w+\s+$([regex]::Escape($archive))$" })
if ($checksumEntries.Count -ne 1) { throw "Expected one SHA256SUMS entry for $archive" }
$expected = ($checksumEntries[0] -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw "SHA-256 mismatch for $archive" }
Expand-Archive -Path $archive -DestinationPath .
$bundleDir = [IO.Path]::GetFileNameWithoutExtension($archive)
$installDir = Join-Path $env:LOCALAPPDATA "Programs\potoo-kalimat"
New-Item -ItemType Directory -Force $installDir | Out-Null
Copy-Item (Join-Path $bundleDir "potoo-kalimat.exe") $installDir
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ';') -notcontains $installDir) {
  $updatedPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $installDir } else { "$userPath;$installDir" }
  [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
}
$env:Path = "$installDir;$env:Path"
```

The Windows commands add the user-writable install directory to your user `PATH`; open a new terminal to use the command in future sessions.

## References

- [Official Kalimat site](https://kalimat-lang.com/)
- [Official Kalimat FAQ](https://kalimat-lang.com/faq.html), which describes C FFI among the language capabilities
- [Official getting-started guide](https://kalimat-lang.com/getting-started.html), which describes the grammar PDFs included with the distribution.
- [Creator's March 2013 article](https://iamsamy.blogspot.com/2013/03/kalimat-educational-programming.html), a historical source for additional features whose current status is not established by the bundled PDFs.

## Support

If this project is useful to you, you can support its continued development:

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## Project policies

- [Code of conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)
