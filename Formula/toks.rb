class Toks < Formula
  desc "Agent‑agnostic CLI output compressor (Token Slim)"
  homepage "https://github.com/arkaung/toks"
  # <<< UPDATE >>>
  url "https://github.com/arkaung/toks/archive/refs/tags/v0.0.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256_OF_SOURCE_TARBALL"
  # >>> UPDATE >>>
  license "MIT"

  depends_on "rust" => :build

  # <<< BOTTLE UPDATE >>>
  bottle do
    root_url "https://github.com/arkaung/toks/releases/download/v0.0.0"
    sha256 cellar: :any_skip_relocation, arm64_ventura:   "PLACEHOLDER_ARM64_SHA256"
    sha256 cellar: :any_skip_relocation, x86_64_ventura:   "PLACEHOLDER_INTEL_SHA256"
    sha256 cellar: :any_skip_relocation, x86_64_linux:    "PLACEHOLDER_LINUX_SHA256"
  end
  # >>> BOTTLE UPDATE >>>

  def install
    system "cargo", "build", "--release", "--locked"
    bin.install "target/release/toks"
  end

  def test
    assert_match /Token Slim — agent-agnostic CLI output compressor/,
                 shell_output("#{bin}/toks --help")
    assert_match /^.*\/$/, shell_output("#{bin}/toks ls .")
  end
end