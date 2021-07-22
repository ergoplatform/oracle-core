import ergo.api.ErgoAPI
import ergo.oraclepool._
import org.bouncycastle.crypto.digests.Blake2bDigest

object Main {
  def main(args: Array[String]) = {
    println("Usage:")
    println("java -cp <jarFile> generatePoolToken <recipientAddress>")
    println("java -cp <jarFile> generateOracleToken <recipientAddress> <numOracles>")
    println("java -cp <jarFile> getContractAddresses <oracleTokenId> <poolTokenId>")
    println("java -cp <jarFile> bootStrapPool <oracleTokenId> <poolTokenId> <initialDataPoint_serialized>")
    println("java -cp <jarFile> bootStrapOracle <oracleTokenId> <poolTokenId> <rewardAddress>")
  }
}

object generatePoolToken {
  def main(args: Array[String]): Unit = {
    if (args.size != 1) println("usage: java -cp <jarFile> generatePoolToken <recipientAddress>")
    else {
      val address = args(0)
      val jsonResp = ergo.api.ErgoAPI.issueAsset(address, 1, "POOL", "oracle pool token", 0)
      println(jsonResp)
    }
  }
}

object generateOracleTokens {
  def main(args: Array[String]): Unit = {
    if (args.size != 2) println("usage: java -cp <jarFile> generateOracleToken <recipientAddress> <numOracles>")
    else {
      val address = args(0)
      val count = args(1).toInt
      val jsonResp = ergo.api.ErgoAPI.issueAsset(
        address,
        count,
        "ORACLE",
        "datapoint token",
        0
      )
      println(jsonResp)
    }
  }
}

class Scripts(oracleTokenId: String, poolTokenId: String) {

  lazy val oracleTokenIdBase58 = oracleTokenId.decodeHex.encodeBase58
  lazy val poolTokenIdBase58 = poolTokenId.decodeHex.encodeBase58

  lazy val minPoolBoxValue = minBoxValue + (numOracles + 1) * oracleReward
  val liveEpochScript =
    s"""{ val oldDatapoint = SELF.R4[Long].get;
       |  val delta = oldDatapoint / 100 * $maxOutlierPercent;
       |  val minDatapoint = oldDatapoint - delta;
       |  val maxDatapoint = oldDatapoint + delta;
       |
       |  val oracleBoxes = CONTEXT.dataInputs.filter{(b:Box) =>
       |    b.R5[Coll[Byte]].get == SELF.id &&
       |    b.tokens(0)._1 == fromBase58(\\"$oracleTokenIdBase58\\") &&
       |    b.R6[Long].get >= minDatapoint &&
       |    b.R6[Long].get <= maxDatapoint
       |  };
       |
       |  val pubKey = oracleBoxes.map{(b:Box) => proveDlog(b.R4[GroupElement].get)}(0);
       |
       |  val sum = oracleBoxes.fold(0L, { (t:Long, b: Box) => t + b.R6[Long].get });
       |
       |  val average = sum / oracleBoxes.size;
       |
       |  val oracleRewardOutputs = oracleBoxes.fold((1, true), { (t:(Int, Boolean), b:Box) =>
       |    (t._1 + 1, t._2 &&
       |               OUTPUTS(t._1).propositionBytes == proveDlog(b.R4[GroupElement].get).propBytes &&
       |               OUTPUTS(t._1).value >= $oracleReward)
       |  });
       |
       |  val epochPrepScriptHash = SELF.R6[Coll[Byte]].get;
       |
       |  sigmaProp(
       |    blake2b256(OUTPUTS(0).propositionBytes) == epochPrepScriptHash &&
       |    oracleBoxes.size > 0 &&
       |    OUTPUTS(0).tokens == SELF.tokens &&
       |    OUTPUTS(0).R4[Long].get == average &&
       |    OUTPUTS(0).R5[Int].get == SELF.R5[Int].get + $postingSchedule &&
       |    OUTPUTS(0).value >= SELF.value - (oracleBoxes.size + 1) * $oracleReward &&
       |    oracleRewardOutputs._2
       |  ) && pubKey
       |}
       |""".stripMargin.lines.mkString

  val liveEpochAddress = ErgoAPI.getP2SAddress(liveEpochScript)
  val liveEpochErgoTree: Array[Byte] = ErgoAPI.getErgoTree(liveEpochAddress).decodeHex
  val digest = new Blake2bDigest(256)
  val liveEpochScriptHash = new Array[Byte](32)
  digest.update(liveEpochErgoTree, 0, liveEpochErgoTree.length)
  digest.doFinal(liveEpochScriptHash, 0)
  val liveEpochScriptHashBase58 = liveEpochScriptHash.encodeBase58

  val epochPrepScript =
    s"""
       |{ val canStartEpoch = HEIGHT > SELF.R5[Int].get - $liveEpochPeriod;
       |  val epochNotOver = HEIGHT < SELF.R5[Int].get;
       |  val epochOver = HEIGHT >= SELF.R5[Int].get;
       |  val enoughFunds = SELF.value >= $minPoolBoxValue;
       |
       |  val maxNewEpochHeight = HEIGHT + $postingSchedule + $buffer;
       |  val minNewEpochHeight = HEIGHT + $postingSchedule;
       |
       |  if (OUTPUTS(0).R6[Coll[Byte]].isDefined) {
       |    val isliveEpochOutput = OUTPUTS(0).R6[Coll[Byte]].get == blake2b256(SELF.propositionBytes) &&
       |                            blake2b256(OUTPUTS(0).propositionBytes) == fromBase58(\\"$liveEpochScriptHashBase58\\");
       |    sigmaProp(
       |      epochNotOver && canStartEpoch && enoughFunds &&
       |      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
       |      OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
       |      OUTPUTS(0).tokens == SELF.tokens &&
       |      OUTPUTS(0).value >= SELF.value &&
       |      isliveEpochOutput
       |    ) || sigmaProp(
       |      epochOver &&
       |      enoughFunds &&
       |      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
       |      OUTPUTS(0).R5[Int].get >= minNewEpochHeight &&
       |      OUTPUTS(0).R5[Int].get <= maxNewEpochHeight &&
       |      OUTPUTS(0).tokens == SELF.tokens &&
       |      OUTPUTS(0).value >= SELF.value &&
       |      isliveEpochOutput
       |    )
       |  } else {
       |    sigmaProp(
       |      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
       |      OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
       |      OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
       |      OUTPUTS(0).tokens == SELF.tokens &&
       |      OUTPUTS(0).value > SELF.value
       |    )
       |  }
       |}
       |""".stripMargin.lines.mkString

  val epochPrepAddress = ErgoAPI.getP2SAddress(epochPrepScript)

  val dataPointScript =
    s"""
       |{ val pubKey = SELF.R4[GroupElement].get;
       |  val liveEpochBox = CONTEXT.dataInputs(0);
       |  val validLiveEpochBox = liveEpochBox.tokens(0)._1 == fromBase58(\\"$poolTokenIdBase58\\") &&
       |                          blake2b256(liveEpochBox.propositionBytes) == fromBase58(\\"$liveEpochScriptHashBase58\\");
       |  sigmaProp(
       |    OUTPUTS(0).R4[GroupElement].get == pubKey &&
       |    OUTPUTS(0).R5[Coll[Byte]].get == liveEpochBox.id &&
       |    OUTPUTS(0).R6[Long].get > 0 &&
       |    OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
       |    OUTPUTS(0).tokens == SELF.tokens &&
       |    validLiveEpochBox
       |  ) && proveDlog(pubKey)
       |}
       |""".stripMargin.lines.mkString

  val dataPointAddress = ErgoAPI.getP2SAddress(dataPointScript)

  val epochPrepErgoTree: Array[Byte] = ErgoAPI.getErgoTree(epochPrepAddress).decodeHex
  val epochPrepScriptHash = new Array[Byte](32)
  digest.update(epochPrepErgoTree, 0, epochPrepErgoTree.length)
  digest.doFinal(epochPrepScriptHash, 0)
  val epochPrepScriptHashBase58 = epochPrepScriptHash.encodeBase58

  val depositScript =
    s"""
       |{
       |  val allFundingBoxes = INPUTS.filter{(b:Box) =>
       |    b.propositionBytes == SELF.propositionBytes
       |  };
       |  val totalFunds = allFundingBoxes.fold(0L, { (t:Long, b: Box) => t + b.value });
       |  sigmaProp(
       |    blake2b256(INPUTS(0).propositionBytes) == fromBase58(\\"$epochPrepScriptHashBase58\\") &&
       |    OUTPUTS(0).propositionBytes == INPUTS(0).propositionBytes &&
       |    OUTPUTS(0).value >= INPUTS(0).value + totalFunds &&
       |    OUTPUTS(0).tokens(0)._1 == fromBase58(\\"$poolTokenIdBase58\\")
       |  )
       |}
       |""".stripMargin.lines.mkString

  val depositAddress = ErgoAPI.getP2SAddress(depositScript)
}

object getContractAddresses {
  def main(args: Array[String]): Unit = {
    if (args.size != 2) println("usage: java -cp <jarFile> getContractAddresses <oracleTokenId> <poolTokenId>")
    else {
      val scripts = new Scripts(oracleTokenId = args(0), poolTokenId = args(1))
      import scripts._
      println
      println(s"LiveEpoch: $liveEpochAddress")
      println
      println(s"EpochPrep: $epochPrepAddress")
      println
      println(s"DataPoint: $dataPointAddress")
      println
      println(s"Deposit: $depositAddress")
      println
      println(s"Min pool box value: $minPoolBoxValue")
    }
  }
}

object bootStrapPool {
  def main(args: Array[String]): Unit = {
    if (args.size != 3)
      println("usage: java -cp <jarFile> boostrapPool <oracleTokenId> <poolTokenId> <initialDataPoint_serialized>")
    else {
      val oracleTokenId = args(0)
      val poolTokenId = args(1)
      val r4: String = args(2) // "0502" // construct using initialDataPoint
      require(r4.take(2) == "05")
      r4.decodeHex // decode to ensure that its a proper hex value

      val scripts = new Scripts(oracleTokenId, poolTokenId)
      import scripts._
      val r5: String = "0480b518" // hardcoded int 200000 (when this epoch ends), much below current height
      val sendRes = ergo.api.ErgoAPI.send(epochPrepAddress, minBoxValue, poolTokenId, 1, Seq(r4, r5))
      println(sendRes)
    }
  }
}

object bootStrapOracle {
  def main(args: Array[String]): Unit = {
    if (args.size != 3) println("usage: java -cp <jarFile> boostrapOracle <oracleTokenId> <poolTokenId> <rewardAddress>")
    else {
      val oracleTokenId = args(0)
      val poolTokenId = args(1)
      val oracleRewardAddress = args(2)
      val eccPoint = ergo.api.ErgoAPI.getEccPoint(oracleRewardAddress)
      val r4: String = "07" + eccPoint
      val r5: String = "0e0101"
      val r6: String = "0504"

      val scripts = new Scripts(oracleTokenId, poolTokenId)
      import scripts._
      val sendRes = ergo.api.ErgoAPI.send(dataPointAddress, minBoxValue, oracleTokenId, 1, Seq(r4, r5, r6))
      println(sendRes)
    }
  }
}
