class Stacy < Formula
  desc "Modern Stata workflow tool for reproducible research"
  homepage "https://github.com/janfasnacht/stacy"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/janfasnacht/stacy/releases/download/v#{version}/stacy-v#{version}-x86_64-apple-darwin.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
    on_arm do
      url "https://github.com/janfasnacht/stacy/releases/download/v#{version}/stacy-v#{version}-aarch64-apple-darwin.tar.gz"
      # sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    url "https://github.com/janfasnacht/stacy/releases/download/v#{version}/stacy-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
    # sha256 "REPLACE_WITH_ACTUAL_SHA256"
  end

  def install
    bin.install "stacy"
  end

  test do
    assert_match "stacy", shell_output("#{bin}/stacy --version")
  end
end
