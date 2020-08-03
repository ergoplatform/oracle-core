{
  // This box:
  // R4: The finalized data point from collection
  // R5: Height the epoch will end

  val canStartEpoch = HEIGHT > SELF.R5[Int].get - livePeriod
  val epochNotOver = HEIGHT < SELF.R5[Int].get
  val epochOver = HEIGHT >= SELF.R5[Int].get
  val enoughFunds = SELF.value >= minPoolBoxValue

  val maxNewEpochHeight = HEIGHT + epochPeriod + buffer
  val minNewEpochHeight = HEIGHT + epochPeriod

  if (OUTPUTS(0).R6[Coll[Byte]].isDefined) {
    val isliveEpochOutput = OUTPUTS(0).R6[Coll[Byte]].get == blake2b256(SELF.propositionBytes) &&
			    blake2b256(OUTPUTS(0).propositionBytes) == liveEpochScriptHash
    sigmaProp( // start next epoch
      epochNotOver && canStartEpoch && enoughFunds &&
      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
      OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
      OUTPUTS(0).tokens == SELF.tokens &&
      OUTPUTS(0).value >= SELF.value &&
      isliveEpochOutput
    ) || sigmaProp( // create new epoch
      epochOver &&
      enoughFunds &&
      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
      OUTPUTS(0).R5[Int].get >= minNewEpochHeight &&
      OUTPUTS(0).R5[Int].get <= maxNewEpochHeight &&
      OUTPUTS(0).tokens == SELF.tokens &&
      OUTPUTS(0).value >= SELF.value &&
      isliveEpochOutput
    )
  } else {
    sigmaProp( // collect funds
      OUTPUTS(0).R4[Long].get == SELF.R4[Long].get &&
      OUTPUTS(0).R5[Int].get == SELF.R5[Int].get &&
      OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
      OUTPUTS(0).tokens == SELF.tokens &&
      OUTPUTS(0).value > SELF.value
    )
  }
}
