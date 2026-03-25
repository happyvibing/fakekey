class Fakekey < Formula
  desc "API Key Proxy Agent - manage and replace API keys via network proxy"
  homepage "https://github.com/happyvibing/fakekey"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-macos-arm64.tar.gz"
      sha256 "" # Updated automatically by release workflow
    else
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-macos-amd64.tar.gz"
      sha256 "" # Updated automatically by release workflow
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-linux-arm64.tar.gz"
      sha256 "" # Updated automatically by release workflow
    else
      url "https://github.com/happyvibing/fakekey/releases/download/v#{version}/fakekey-linux-amd64.tar.gz"
      sha256 "" # Updated automatically by release workflow
    end
  end

  def install
    bin.install "fakekey"
  end

  test do
    system "#{bin}/fakekey", "--version"
  end
end
