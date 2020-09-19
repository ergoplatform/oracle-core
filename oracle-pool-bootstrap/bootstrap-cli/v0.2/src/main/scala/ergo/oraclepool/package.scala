package ergo

import java.io.File

import com.typesafe.config.ConfigFactory
import org.apache.commons.codec.binary.Hex

import scala.language.implicitConversions

package object oraclepool {
  lazy val config = ConfigFactory.parseFile(new File("application.conf")).getConfig("ergo.oraclepool")

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

  lazy val poolConfig = config.getConfig("pool")

  lazy val oracleReward = poolConfig.getLong("oracleReward") // example 2500000 (NanoErgs)
  lazy val minBoxValue = poolConfig.getLong("minBoxValue") // example 1500000L (NanoErgs)
  lazy val maxDeviation = poolConfig.getInt("maxDeviation") // example 50 (percent)
  lazy val numOracles = poolConfig.getInt("numOracles") // example 14

  lazy val minOracleBoxes = poolConfig.getInt("minOracleBoxes") // example 30 (blocks)
  lazy val postingSchedule = poolConfig.getInt("postingSchedule") // example 30 (blocks)
  lazy val liveEpochPeriod = poolConfig.getInt("liveEpochPeriod") // example 20 (blocks) (anything less than postingSchedule)
  // Note that epochPrepPeriod is implicitly computed from above two values using the relation:
  //    postingSchedule = liveEpochPeriod + epochPrepPeriod

  lazy val buffer = poolConfig.getInt("buffer") // example 5 (blocks)
}
