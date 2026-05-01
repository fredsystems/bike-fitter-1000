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

              rustToolchain = pkgs.rust-bin.stable.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              };

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
              pkgs = import nixpkgs {
                inherit system;
                overlays = [ rust-overlay.overlays.default ];
              };
              chk = self.checks.${system}."pre-commit-check";
              corePkgs = chk.enabledPackages or [ ];
              extraDev = chk.passthru.devPackages or [ ];

              # Same toolchain shape as the package build, plus the wasm32
              # target so `cargo build --target wasm32-unknown-unknown`
              # works out of the box for the web crate.
              rustToolchain = pkgs.rust-bin.stable.latest.default.override {
                targets = [ "wasm32-unknown-unknown" ];
              };

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
                  # Drop the precommit flake's bundled rustc/cargo/clippy/
                  # rustfmt; we provide our own toolchain (with the wasm32
                  # target) and the precommit hooks pick it up via PATH.
                  (builtins.filter (
                    p:
                    !(builtins.elem (p.pname or "") [
                      "rustc"
                      "cargo"
                      "clippy"
                      "rustfmt"
                      "rust-default"
                    ])
                  ) (extraDev ++ corePkgs))
                  ++ [
                    rustToolchain
                    pkgs.pkg-config
                    # Wasm pipeline: trunk drives the build; wasm-bindgen-cli
                    # generates the JS shim. binaryen's wasm-opt is what
                    # trunk shells out to for release-mode minification.
                    pkgs.trunk
                    pkgs.wasm-bindgen-cli
                    pkgs.binaryen
                  ];

                LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;

                shellHook = ''
                  ${chk.shellHook}
                  # Pre-pend our pinned rust toolchain so its rustc/cargo/
                  # clippy/rustfmt take precedence over the (older) ones the
                  # precommit flake exposes via its own shellHook. The two
                  # must agree on rustc version or clippy fails to load
                  # crate metadata built by cargo (E0514).
                  export PATH=${rustToolchain}/bin:$PATH
                  alias pre-commit="pre-commit run --all-files"
                '';
              };
            };
        }) systems
      );
    };
}
