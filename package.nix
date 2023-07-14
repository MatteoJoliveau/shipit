{ naersk
, version
, pkgs
, ...
}:

naersk.buildPackage {
  name = "shipit";
  inherit version;

  src = ./.;

  nativeBuildInputs = with pkgs; [
    clang
  ];

  buildInputs = with pkgs; [ ];
}
