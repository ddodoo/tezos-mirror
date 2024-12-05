(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* Copyright (c) 2024 Nomadic Labs, <contact@nomadic-labs.com>               *)
(*                                                                           *)
(*****************************************************************************)

(** This module handles the logic of flags for AI/AS in the [State] *)

module AI_Activation : sig
  (** This module takes care of the flags [force_activation] and
      [activation_vote_enable], and updates and check the record field
      [ai_activation_cycle] in the state. *)

  (** [AI_Activation] is enabled iff either of the above flags are set to true,
      or the vote threshold is set to 0 (regardless of votes).
      "Enabled" here means that AI *can* be activated (either by vote
      or by force), but does not mean that AI is activated *)
  val enabled : State.t -> bool

  (** Checks the [ai_activation_cycle] is set as expected.
      Run at the beginning, and after every block to check the activation
      cycle is set only once ever. *)
  val check_activation_cycle : Block.t -> State.t -> State.t tzresult Lwt.t
end

module AI : sig
  (** This module only checks (for now) if AI is activated or not *)

  (** AI is enabled iff the activation cycle is set and passed *)
  val enabled : Block.t -> State.t -> bool
end

module Delayed_slashing : sig
  (** This module takes care of choosing the denunciations that need to be
      applied at the end of a cycle. It depends on the flag [ns_enable]. *)

  (** [partition_slashes s cycle] returns a pair [(l1,l2)] of lists of slashes,
      partitioned from the [state.pending_slashes]. [l2] is the list of slashes to
      apply at the end of the given [cycle], and [l1] is the rest (which should
      usually replace [state.pending_slashes])
      *)
  val partition_slashes :
    State.t ->
    Protocol.Alpha_context.Cycle.t ->
    (Signature.Public_key_hash.t * Protocol.Denunciations_repr.item) list
    * (Signature.Public_key_hash.t * Protocol.Denunciations_repr.item) list
end
