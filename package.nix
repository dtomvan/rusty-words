{
  lib,
  rustPlatform,
  versionCheckHook,
}:
rustPlatform.buildRustPackage rec {
  pname = "rusty-words";
  version = "0.1.1";

  src = ./.;

  cargoHash = "sha256-/hbRzl8dh6r1mRbW1RZ2idhOJDP7HJQX6R7LpSn1xIw=";

  doInstallCheck = true;
  nativeInstallCheckInputs = [ versionCheckHook ];
  versionCheckProgram = "${placeholder "out"}/bin/${meta.mainProgram}";

  meta = {
    description = "Practice your flashcards like in Quizlet, but for the TUI";
    homepage = "https://github.com/dtomvan/rusty-words";
    license = lib.licenses.mit;
    mainProgram = "rwds-cli";
  };
}
