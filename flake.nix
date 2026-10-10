{
  description = "symGCAD development and native upstream comparison environments";
  inputs.nixpkgs.url = "nixpkgs";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f (import nixpkgs { inherit system; }));
    in {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [ git rustc cargo rustfmt clippy gcc gnumake m4 pkg-config gmp mpfr time hyperfine perf valgrind typst noto-fonts libertinus ];
          CARGO_BUILD_JOBS = "16";
          RUST_BACKTRACE = "1";
          TYPST_FONT_PATHS = "${pkgs.libertinus}/share/fonts:${pkgs.noto-fonts}/share/fonts";
          shellHook = ''
            if [ -f .symgcad-env ]; then source .symgcad-env; fi
          '';
        };
        reference = pkgs.mkShell {
          packages = with pkgs; [ git gcc gnumake pkg-config flint gmp mpfr time (python3.withPackages (ps: with ps; [ sympy cython setuptools setuptools-scm pytest pip numpy scipy networkx pyyaml ])) ];
        };
      });
    };
}
