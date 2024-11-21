(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* SPDX-FileCopyrightText: 2024 Nomadic Labs <contact@nomadic-labs.com>      *)
(*                                                                           *)
(*****************************************************************************)

(** [register_backend identifiers profiler] associates one or more
    identifier(s) with a profiler instantiator.

    e.g.
    {[
    let () = register_backend ["json"] write_to_json_file

    let () = register_backend ["text"; "txt"] write_to_txt_file
    ]}

    The identifiers are matched against the PROFILING_BACKEND environment
    variable in order to be selected when profiling.

    This module registers two default profilers defineds in {!Simple_profiler}.
    One for [PROFILING_BACKEND=json] which output raw data to json files, and
    one for [PROFILING_BACKEND=txt] and [PROFILING_BACKEND=text] that output
    plain text formatted reports.

    Note that these profilers add [_profiling.txt] or [_profiling.json]
    suffix to the file name. *)
val register_backend :
  string list ->
  (verbosity:Profiler.verbosity ->
  directory:string ->
  name:string ->
  Profiler.instance) ->
  unit

(** [selected_backend ()] returns the backend selected using the environment
    variable [PROFILING_BACKEND]. *)
val selected_backend :
  unit -> (directory:string -> name:string -> Profiler.instance option) option
