{
  description = "bike-fitter-1000 — bike fit solver and side-view visualizer";

  inputs = {
    precommit.url = "github:FredSystems/pre-commit-checks";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      precommit,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systems = precommit.lib.supportedSystems;
    in
    {
      ##########################################################################
      ## PACKAGES — build the GUI binary
      ##########################################################################
      packages = builtins.listToAttrs (
        map (system: {
          name = system;
          value =
            let
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
              };

              rustToolchain = pkgs.rust-bin.stable.latest.default;

              rustPlatform = pkgs.makeRustPlatform {
                cargo = rustToolchain;
                rustc = rustToolchain;
              };

              # Runtime libs needed by egui/winit/wgpu on Linux.
              runtimeLibs = [
                pkgs.libxkbcommon
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
                pkgs.wayland
                pkgs.libGL
                pkgs.vulkan-loader
              ];

              runtimeLibPath = pkgs.lib.makeLibraryPath runtimeLibs;
            in
            {
              default = rustPlatform.buildRustPackage {
                pname = "bike-fitter-1000";
                version = "0.1.0";
                src = pkgs.lib.cleanSource ./.;
                cargoLock.lockFile = ./Cargo.lock;

                nativeBuildInputs = [
                  pkgs.pkg-config
                  pkgs.makeWrapper
                ];
                buildInputs = runtimeLibs;

                postInstall = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                  wrapProgram $out/bin/bike-fitter-1000 \
                    --prefix LD_LIBRARY_PATH : ${runtimeLibPath}
                '';
              };
            };
        }) systems
      );

      ##########################################################################
      ## CHECKS — pre-commit (base + rust)
      ##########################################################################
      checks = builtins.listToAttrs (
        map (system: {
          name = system;
          value = {
            pre-commit-check = precommit.lib.mkCheck {
              inherit system;
              src = ./.;
              check_rust = true;
              enableXtask = false;
              extraExcludes = [
                "^docs/"
                "^data/"
              ];
            };
          };
        }) systems
      );

      ##########################################################################
      ## DEV SHELLS
      ##########################################################################
      devShells = builtins.listToAttrs (
        map (system: {
          name = system;
          value =
            let
              pkgs = import nixpkgs { inherit system; };
              chk = self.checks.${system}."pre-commit-check";
              corePkgs = chk.enabledPackages or [ ];
              extraDev = chk.passthru.devPackages or [ ];

              runtimeLibs = [
                pkgs.libxkbcommon
                pkgs.libGL
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
                pkgs.wayland
                pkgs.vulkan-loader
              ];
            in
            {
              default = pkgs.mkShell {
                buildInputs =
                  extraDev
                  ++ corePkgs
                  ++ [
                    pkgs.pkg-config
                  ];

                LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

                shellHook = ''
                  ${chk.shellHook}
                  alias pre-commit="pre-commit run --all-files"
                '';
              };
            };
        }) systems
      );
    };
}
