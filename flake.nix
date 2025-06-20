{
  description = "A flake providing a Python development shell with uv";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
      in {
        pkgs.config.allowUnfree = true;
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.python312
            pkgs.uv
            pkgs.claude-code
          ];
          shellHook = ''
            echo "Python: $(python --version)"
            echo "uv: $(uv --version)"
          '';
        };
      }
    );
}
