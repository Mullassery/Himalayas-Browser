class Himalayas < Formula
  desc "Agent-native browser daemon: fast, private, automation-first"
  homepage "https://github.com/Mullassery/Himalayas-Browser"
  license "Proprietary"
  head "https://github.com/Mullassery/Himalayas-Browser.git", branch: "main"

  # HEAD-only for now: no tagged release with a hosted, checksummed source
  # tarball exists yet (see README.md's "Install in 30 Seconds" and
  # docs/GETTING_STARTED.md). Once one does, add a stable `url`/`sha256`
  # block above pointing at that release tag and this comment can go —
  # `head` and a stable block can coexist in the same formula.
  #
  # This repo isn't named `homebrew-<tap>`, so `brew tap` needs the full
  # URL the first time:
  #   brew tap mullassery/himalayas-browser https://github.com/Mullassery/Himalayas-Browser
  #   brew install --HEAD himalayas
  #
  # Only builds the `himalayas` binary (the daemon / `/app` web-shell
  # client) — `himalayas-desktop` (the native GPU-rendered shell spike,
  # docs/NATIVE_RENDERING_PLAN.md) sits behind the `js_engine` Cargo
  # feature, which is off by default and pulls in a much larger dependency
  # tree (winit/wgpu/vello/stylo/boa); `cargo install` only builds binaries
  # whose `required-features` are satisfied by the enabled feature set, so
  # it's skipped here without needing an explicit exclusion.

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match "Himalayas Browser", shell_output("#{bin}/himalayas --help")
  end
end
