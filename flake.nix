{
  description = "URL shortener Service";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Korrekte Anwendung des rust-overlay
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Definiere die Rust-Toolchain (Version ggf. anpassen)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override { 
           extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Systemabhängigkeiten, die zum Bauen benötigt werden
        nativeBuildInputs = with pkgs; [
          pkg-config 
        ];
        buildInputs = with pkgs; [
          sqlite    
          openssl    
          # Füge hier weitere C-Bibliotheken hinzu, falls deine Crates sie brauchen
        ];

      in
      {
        # Entwicklungsumgebung (für `nix develop`)
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain        
            pkgs.sqlite-interactive 
          ] ++ nativeBuildInputs ++ buildInputs; 

          # Hier kannst du Umgebungsvariablen für die Dev-Shell setzen (optional)
          # shellHook = ''
          #  export DATABASE_URL="sqlite://./shortener.db"
          # '';
        };

        # Paketdefinition (für `nix build` und `nix run`)
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "rurl"; 
          version = "0.1.0";     

          src = ./.; 

          # WICHTIG: Cargo.lock wird für reproduzierbare Builds benötigt!
          cargoLock.lockFile = ./Cargo.lock;

          # Systemabhängigkeiten, die von `cargo build` benötigt werden
          nativeBuildInputs = nativeBuildInputs; 
          buildInputs = buildInputs;            


        };
      }
    );
}