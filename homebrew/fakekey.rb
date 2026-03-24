class Fakekey < Formula
  desc "API Key Proxy Agent - manage and replace API keys via network proxy"
  homepage "https://github.com/happyvibing/fakekey"
  url "https://github.com/happyvibing/fakekey/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "" # Update this after release
  license "Apache-2.0"

  depends_on "rust"

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/fakekey", "--version"
  end
end
