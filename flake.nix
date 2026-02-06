{
  description = "CLI tool for managing Portainer stacks";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "stack-sync";
          version = self.shortRev or self.dirtyShortRev or "dev";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          meta = with pkgs.lib; {
            description = "A CLI tool for managing Portainer stacks";
            homepage = "https://github.com/kyeotic/stack-sync";
            license = licenses.mit;
            mainProgram = "stack-sync";
          };
        };
      }
    );
}
