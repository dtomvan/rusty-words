{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "rusty-words";
  version = "0.1.0";

  src = ./.;

  cargoHash = "sha256-y4ezjzNuJTa3OobX8G093wKJ6eieY9DMaM/ePEx8B6U=";

  meta = {
    description = "Practice your flashcards like in Quizlet, but for the TUI";
    homepage = "https://github.com/dtomvan/rusty-words";
    license = lib.licenses.mit;
    mainProgram = "rwds-cli";
  };
}
