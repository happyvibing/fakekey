class Fakekey < Formula
  desc "API Key Proxy Agent - manage and replace API keys via network proxy"
  homepage "https://github.com/happyvibing/fakekey"
  url "https://crates.io/api/v1/crates/fakekey/0.1.0/download"
  sha256 "e99c9d5bc1b6a6e85ee5c5bbcb6debfdb18eab2dc07f0b31b078deeea7a062d2"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/fakekey", "--version"
  end
end
