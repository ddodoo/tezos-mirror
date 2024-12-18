(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* SPDX-FileCopyrightText: 2024 Nomadic Labs <contact@nomadic-labs.com>      *)
(*                                                                           *)
(*****************************************************************************)

module Agent = Agent
module Alert_manager = Alert_manager

module Configuration : sig
  type docker_image =
    | Gcp of {alias : string}
    | Octez_release of {tag : string}

  type t = private {
    machine_type : string;
    docker_image : docker_image;
    max_run_duration : int option;
    binaries_path : string;
    os : string;
  }

  (** [make ?machine_type ()] is a smart-constructor to make a VM
      configuration.

    Default value for [max_run_duration] is [7200].

    Default value for [machine_type] is [n1-standard-2].

    Default value for [docker_image] is [Custom {tezt_cloud}] where [tezt_cloud]
    is the value provided by the environement variable [$TEZT_CLOUD].
    *)
  val make :
    ?os:string ->
    ?binaries_path:string ->
    ?max_run_duration:int ->
    ?machine_type:string ->
    ?docker_image:docker_image ->
    unit ->
    t
end

module Cloud : sig
  type t

  (** A wrapper around [Test.register] that can be used to register new tests
      using VMs provided as a map indexed by name. Each VM is abstracted via
      the [Agent] module.

      [proxy_files] should contains [file] that are needed by the
      scenario to run (only used for proxy mode).

      [proxy_args] should contains CLI arguments necessary for the
      proxy mode. This can be used for example when an argument is
      provided via an environment variable instead of a command-line
      argument.
 *)
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

  (** [push_metric t ?help ?typ ?labels ~name v] pushes the value [v]
      for [metric] on Prometheus. [labels] can be used to categorise
      the metric (each set of label define a single curve). [typ] can
      be used to provide the type of the metric. [help] can be used to
      provide some naive documentation about the metrics. *)
  val push_metric :
    t ->
    ?help:string ->
    ?typ:[`Counter | `Gauge] ->
    ?labels:(string * string) list ->
    name:string ->
    float ->
    unit

  (** [agents t] returns the list of agents deployed. *)
  val agents : t -> Agent.t list

  (** [set_agent_name t agent name] sets the name of the agent [agent] to
      [name]. *)
  val set_agent_name : t -> Agent.t -> string -> unit Lwt.t

  type target = {agent : Agent.t; port : int; app_name : string}

  (** [add_prometheus_source t ?metrics_path ~name targets] allows to add a new
      source of metrics that Prometheus can scrap. By default [metric_path] is
      [/metrics]. [job_name] is just the name to give for the job that will
      scrap the metrics. It must be unique. A target enables to define a list of
      points to scrap. Each point can have a name defined by [app_name]. *)
  val add_prometheus_source :
    t -> ?metrics_path:string -> name:string -> target list -> unit Lwt.t

  val add_service : t -> name:string -> url:string -> unit Lwt.t
end

(** [register ~tags] register a set of jobs that can be used for setting
   requirements related to cloud scenarios. Some tags can be given for all the
   registered jobs. *)
val register : tags:string list -> unit
