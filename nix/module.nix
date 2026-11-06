{
  self ? null,
}:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.programs.aquilum;
  system = pkgs.stdenv.hostPlatform.system;

  defaultPackage =
    if self == null then pkgs.aquilum else self.packages.${system}.aquilum or pkgs.aquilum;
in
{
  options.programs.aquilum = {
    enable = lib.mkEnableOption "Aquilum, a local-first Markdown knowledge base";

    package = lib.mkOption {
      type = lib.types.package;
      default = defaultPackage;
      defaultText = lib.literalExpression "inputs.aquilum.packages.${pkgs.stdenv.hostPlatform.system}.aquilum";
      description = ''
        The Aquilum package to install. The default comes from this flake; it
        can be overridden with another build of the application.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
  };
}
