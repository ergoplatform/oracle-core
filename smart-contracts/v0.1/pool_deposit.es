{
  val allFundingBoxes = INPUTS.filter{(b:Box) =>
    b.propositionBytes == SELF.propositionBytes
  }

  val totalFunds = allFundingBoxes.fold(0L, { (t:Long, b: Box) => t + b.value })

  sigmaProp(
    blake2b256(INPUTS(0).propositionBytes) == epochPrepScriptHash &&
    OUTPUTS(0).propositionBytes == INPUTS(0).propositionBytes &&
    OUTPUTS(0).value >= INPUTS(0).value + totalFunds &&
    OUTPUTS(0).tokens(0)._1 == poolTokenId
  )
}
