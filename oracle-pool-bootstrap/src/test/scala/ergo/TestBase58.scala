package ergo

import ergo.oraclepool._

object TestBase58 {
  def main(args: Array[String]): Unit = {
    val hex = "12caaacb51c89646fac9a3786eb98d0113bd57d68223ccc11754a4f67281daed"
    val bytes = hex.decodeHex
    val base58 = bytes.encodeBase58
    val newBytes = base58.decodeBase58
    val newHex = newBytes.encodeHex
    assert(newHex == hex)
    assert(base58 == "2GMaYqCG3Hq2xMwiDiA2PuYx2UVLB2EijVrsCMoEyj6L")
  }
}
