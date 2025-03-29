{
  description = "URL shortener Service";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url  = "github:numtide/flake-utils";
    postgres = {
      url = "github:Datata1/my_flakes";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, postgres, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # TODO: latest ist impure, besser wäre ein statischer Wert
        rustToolchain = pkgs.rust-bin.stable.latest.default.override { 
           extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          pkg-config 
        ];

        buildInputs = with pkgs; [
          postgresql.lib    
          openssl
          sqlx-cli    
        ];

        postgresAppProgram = postgres.apps.${system}.postgres.program;

        rurlPackage = self.packages.${system}.default;

        processComposeConfigFile = ./process-compose.yml;

      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain        
            pkgs.sqlite-interactive
            pkgs.postgresql
            pkgs.process-compose 
          ] ++ nativeBuildInputs ++ buildInputs; 
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "rurl";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs;
        };

        packages.start-dev = pkgs.writeShellScriptBin "start-dev" ''
          #!${pkgs.runtimeShell}
          set -e # Beenden bei Fehlern

          export DB_COMMAND="${postgresAppProgram}"
          export APP_COMMAND="${rurlPackage}/bin/rurl"
          export DB_USER=$(whoami)
          export DB_DATABASE=$(whoami)
          echo $APP_COMMAND
          export DATABASE_URL="postgres://''$(whoami):devpassword@localhost:5432/''$(whoami)"
          CONFIG_FILE="${processComposeConfigFile}"

          export PATH=${pkgs.lib.makeBinPath [ pkgs.postgresql pkgs.process-compose ]}:$PATH

          echo "Starting services with process-compose..."
          ${pkgs.process-compose}/bin/process-compose -f "$CONFIG_FILE"
        '';

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.start-dev}/bin/start-dev";
        };

        defaultApp = self.apps.${system}.default;

      }
    );
}