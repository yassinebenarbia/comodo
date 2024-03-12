{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        unstable = import <nixpkgs-unstable> {};
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        shellHook = ''
        export PKG_CONFIG_PATH=${pkgs.alsa-lib.dev}/lib/pkgconfig/
        export PATH=${pkgs.pkg-config}/bin:$PATH
        '';
        devShell = with pkgs; mkShell {
          buildInputs = [ pkg-config alsa-lib.dev gcc unstable.rustc ];
        };
      });
}
