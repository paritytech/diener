class Diener < Formula
  desc "dependency diener is a tool for easily changing Substrate or Polkadot dependency versions"
  homepage "https://github.com/paritytech/diener"
  url "https://github.com/paritytech/diener/releases/download/v0.4.4/diener_macos_v0.4.4.tar.gz"
  sha256 "920fa14badc091a0cb7e89e79fb5ade3205f721b489b2f27640b5963bb670693"
  version "0.4.4"

  def install
    bin.install "diener"
  end
end

