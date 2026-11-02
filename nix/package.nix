{ lib, rustPlatform }:

let
  cargo = lib.importTOML ../Cargo.toml;
  manifest = lib.importTOML ../herdr-plugin.toml;
  version = cargo.workspace.package.version;
in
assert lib.assertMsg (
  manifest.version == version
) "herdr-plugin.toml says ${manifest.version}, Cargo.toml says ${version}";

rustPlatform.buildRustPackage {
  pname = "herdr-needs-action";
  inherit version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../crates
      ../herdr-plugin.toml
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  # The manifest runs the binary from where `cargo build --release` leaves
  # it, so that one manifest serves `herdr plugin install`, a link to a
  # checkout, and this package.
  postInstall = ''
    root=$out/share/herdr-plugins/needs-action
    install -Dm644 herdr-plugin.toml $root/herdr-plugin.toml
    mkdir -p $root/target/release
    ln -s $out/bin/herdr-needs-action $root/target/release/herdr-needs-action
  '';

  passthru.pluginPath = "share/herdr-plugins/needs-action";

  meta = {
    description = manifest.description;
    homepage = cargo.workspace.package.repository;
    mainProgram = "herdr-needs-action";
    platforms = lib.platforms.linux ++ lib.platforms.darwin;
  };
}
