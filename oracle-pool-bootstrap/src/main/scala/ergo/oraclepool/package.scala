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

  val oracleReward = 2500000L // NanoErgs
  val minBoxValue = 1500000L // NanoErgs
  val errorMargin = 50 // percent
  val numOracles = 14
  val epochPeriod = 30 // blocks
  val livePeriod = 20 // blocks
  val buffer = 5 // blocks
}
