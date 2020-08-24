package ergo

import org.apache.commons.codec.binary.Hex
import scala.language.implicitConversions

package object oraclepool {
  class BetterString(string: String) {
    def decodeHex: Array[Byte] = Hex.decodeHex(string.toCharArray)
    def decodeBase58: Array[Byte] = Base58Check.decodePlain(string)
  }

  class BetterByteArray(bytes: Seq[Byte]) {
    def encodeHex: String = Hex.encodeHexString(bytes.toArray).toLowerCase
    def encodeBase58: String = Base58Check.encodePlain(bytes.toArray)
  }

  implicit def ByteArrayToBetterByteArray(bytes: Array[Byte]): BetterByteArray = new BetterByteArray(bytes)

  implicit def StringToBetterString(string: String): BetterString = new BetterString(string)

  lazy val oracleReward = 2500000L // NanoErgs
  lazy val minBoxValue = 1500000L // NanoErgs
  lazy val errorMargin = 50 // percent
  lazy val numOracles = 14
  lazy val epochPeriod = 30 // blocks
  lazy val livePeriod = 20 // blocks
  lazy val buffer = 5 // blocks
}
