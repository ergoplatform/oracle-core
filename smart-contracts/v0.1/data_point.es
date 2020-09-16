{
 // This box:
 // R4: The address of the oracle (never allowed to change after bootstrap).
 // R5: The box id of the latest Live Epoch box.
 // R6: The oracle's datapoint.

 val pubKey = SELF.R4[GroupElement].get

 val liveEpochBox = CONTEXT.dataInputs(0)

 val validLiveEpochBox = liveEpochBox.tokens(0)._1 == poolTokenId &&
			 blake2b256(liveEpochBox.propositionBytes) == liveEpochScriptHash

 sigmaProp(
   OUTPUTS(0).R4[GroupElement].get == pubKey &&
   OUTPUTS(0).R5[Coll[Byte]].get == liveEpochBox.id &&
   OUTPUTS(0).R6[Long].get > 0 &&
   OUTPUTS(0).propositionBytes == SELF.propositionBytes &&
   OUTPUTS(0).tokens == SELF.tokens &&
   validLiveEpochBox
 ) && proveDlog(pubKey)
}
