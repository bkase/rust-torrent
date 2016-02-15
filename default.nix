with import <nixpkgs> { };

stdenv.mkDerivation rec {
  name = "rust-torrent-${version}";
  version = "0.1.0";

  buildInputs = [ rustPlatform.rustc cargo openssl ];
}

