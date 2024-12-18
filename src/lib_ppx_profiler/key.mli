(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* SPDX-FileCopyrightText: 2024 Nomadic Labs <contact@nomadic-labs.com>      *)
(*                                                                           *)
(*****************************************************************************)

type content =
  | Empty
  | Ident of string
  | String of string
  | List of Ppxlib.expression list
  | Apply of Ppxlib.expression * Ppxlib.expression list
  | Other of Ppxlib.expression

type t = {
  verbosity : string option;
  profiler_module : string option;
  metadata : Ppxlib.expression option;
  content : content;
}

val get_verbosity : Ppxlib.Location.t -> t -> Ppxlib.expression option

val get_profiler_module : t -> Longident.t

val to_expression : Ppxlib.Location.t -> t -> Ppxlib.expression

val pp : Format.formatter -> t -> unit

val content : t -> content
