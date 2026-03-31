class Fakekey < Formula
  desc "API Key Proxy Agent - manage and replace API keys via network proxy"
  homepage "https://github.com/happyvibing/fakekey"
  version "0.1.1"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-macos-arm64.tar.gz"
      sha256 "6c88e71496427f36e9a69fd6dbe4d532bb9ef7d6a2105bd30115f25489c8ad0b"
    end
    on_intel do
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-macos-amd64.tar.gz"
      sha256 "5c4760b266e0e21a068fa63506762ef3e2712ad5f4c121791af9ab7d2ace1607"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-linux-arm64.tar.gz"
      sha256 "7b09afdd1cc5424331fb8bcdda4b5113581bccfea7f4559380a216ce96e0f41e"
    end
    on_intel do
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-linux-amd64.tar.gz"
      sha256 "a6f6a2e3f5cf7a501904d82e28cd68224e4b6ea37e305add22ce9d32a32187cd"
    end
  end

  def install
    bin.install "fakekey"
  end

  test do
    system "#{bin}/fakekey", "--version"
  end
end
