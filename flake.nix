{
  description = "Aquilum — fast local-first Markdown knowledge base";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system} system);
    in
    {
      packages = forAllSystems (
        pkgs: _:
        let
          aquilum = pkgs.callPackage ./nix/package.nix { };
        in
        {
          inherit aquilum;
          default = aquilum;
        }
      );

      apps = forAllSystems (
        _: system:
        let
          app = {
            type = "app";
            program = "${self.packages.${system}.aquilum}/bin/aquilum-app";
            meta.description = "Fast local-first Markdown knowledge base";
          };
        in
        {
          aquilum = app;
          default = app;
        }
      );

      checks = forAllSystems (
        _: system: {
          inherit (self.packages.${system}) aquilum;
        }
      );

      overlays.default = final: _prev: {
        aquilum = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = import ./nix/module.nix { inherit self; };

      devShells = forAllSystems (
        pkgs: _:
        let
          inherit (pkgs) lib;
          gstreamerPlugins = with pkgs.gst_all_1; [
            gst-plugins-base
            gst-plugins-good
            gst-plugins-bad
            gst-libav
          ];
        in
        {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.aquilum ];

            packages = with pkgs; [
              cargo
              clippy
              rustc
              rustfmt
              nixfmt
            ];

            shellHook = ''
              export CARGO_TARGET_DIR="$PWD/.artifacts/cargo-native"
              export GST_PLUGIN_SYSTEM_PATH_1_0="${lib.makeSearchPath "lib/gstreamer-1.0" gstreamerPlugins}''${GST_PLUGIN_SYSTEM_PATH_1_0:+:$GST_PLUGIN_SYSTEM_PATH_1_0}"
              echo "Aquilum development shell. Build with: cd aquilum-app && cargo build -p aquilum-native"
            '';
          };
        }
      );

      formatter = forAllSystems (pkgs: _: pkgs.nixfmt);
    };
}
