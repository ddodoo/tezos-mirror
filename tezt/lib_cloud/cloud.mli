(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* SPDX-FileCopyrightText: 2024 Nomadic Labs <contact@nomadic-labs.com>      *)
(*                                                                           *)
(*****************************************************************************)

type t

(** [register ?proxy_files ?vms ~__FILE__ ~title ~tags ?seed f] is a
    wrapper around [Test.register]. It enables to run a test that can
    use machines deployed onto the cloud. *)
val register :
  ?proxy_files:string list ->
  ?proxy_args:string list ->
  ?vms:Configuration.t list ->
  __FILE__:string ->
  title:string ->
  tags:string list ->
  ?seed:Test.seed ->
  ?alert_collection:Alert_manager.Collection.t ->
  (t -> unit Lwt.t) ->
  unit

val agents : t -> Agent.t list

val get_configuration : Agent.t -> Configuration.t

val push_metric :
  t ->
  ?help:string ->
  ?typ:[`Counter | `Gauge] ->
  ?labels:(string * string) list ->
  name:string ->
  float ->
  unit

val write_website : t -> unit Lwt.t

val set_agent_name : t -> Agent.t -> string -> unit Lwt.t

type target = {agent : Agent.t; port : int; app_name : string}

val add_prometheus_source :
  t -> ?metrics_path:string -> name:string -> target list -> unit Lwt.t

val add_service : t -> name:string -> url:string -> unit Lwt.t
