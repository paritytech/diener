class Diener < Formula
  desc "dependency diener is a tool for easily changing Substrate or Polkadot dependency versions"
  homepage "https://github.com/paritytech/diener"
  url "https://github.com/paritytech/diener/releases/download/v0.4.4/diener_macos_v0.4.4.tar.gz"
  sha256 "1caba305bcc460a528fbcf2f0a7dd519fb31ad21fffa9902d9497019809ff90b"
  version "0.4.4"

  def install
    bin.install "diener"
  end
end

