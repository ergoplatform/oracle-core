{ // This box:
  // R4: The latest finalized datapoint (from the previous epoch)
  // R5: Block height that the current epoch will finish on
  // R6: Address of the "Epoch Preparation" stage contract.

  // Oracle box:
  // R4: Public key (group element)
  // R5: Epoch box Id (this box's Id)
  // R6: Data point

  val oracleBoxes = CONTEXT.dataInputs.filter{(b:Box) =>
    b.R5[Coll[Byte]].get == SELF.id &&
    b.tokens(0)._1 == oracleTokenId
  }

  val pubKey = oracleBoxes.map{(b:Box) => proveDlog(b.R4[GroupElement].get)}(0)

  val sum = oracleBoxes.fold(0L, { (t:Long, b: Box) => t + b.R6[Long].get })

  val average = sum / oracleBoxes.size // do we need to check for division by zero here?

  val oracleRewardOutputs = oracleBoxes.fold((1, true), { (t:(Int, Boolean), b:Box) =>
    (t._1 + 1, t._2 &&
               OUTPUTS(t._1).propositionBytes == proveDlog(b.R4[GroupElement].get).propBytes &&
               OUTPUTS(t._1).value >= oracleReward)
  })

  val epochPrepScriptHash = SELF.R6[Coll[Byte]].get

  sigmaProp(
    blake2b256(OUTPUTS(0).propositionBytes) == epochPrepScriptHash &&
    oracleBoxes.size > 0 &&
    OUTPUTS(0).tokens == SELF.tokens &&
    OUTPUTS(0).R4[Long].get == average &&
    OUTPUTS(0).R5[Int].get == SELF.R5[Int].get + epochPeriod &&
    OUTPUTS(0).value >= minPoolBoxValue &&
    OUTPUTS(0).value >= SELF.value - (oracleBoxes.size + 1) * oracleReward &&
    oracleRewardOutputs._2
  ) && pubKey
}
