(*****************************************************************************)
(*                                                                           *)
(* SPDX-License-Identifier: MIT                                              *)
(* SPDX-FileCopyrightText: 2024 Nomadic Labs <contact@nomadic-labs.com>      *)
(*                                                                           *)
(*****************************************************************************)

type error =
  | Invalid_action of string
  | Invalid_payload
  | Invalid_aggregate of Key.t
  | Invalid_custom of Key.t
  | Invalid_mark of Key.t
  | Invalid_record of Key.t
  | Invalid_span of Key.t
  | Invalid_stop of Key.t
  | Improper_field of (Longident.t Location.loc * Ppxlib.expression)
  | Improper_let_binding of Ppxlib.expression
  | Improper_record of (Ppxlib.Ast.longident_loc * Ppxlib.expression) list
  | Malformed_attribute of Ppxlib.expression
  | No_verbosity of Key.t

let pp_field ppf (lident, expr) =
  Format.fprintf
    ppf
    "%s = %a"
    (Ppxlib.Longident.name lident.Ppxlib.txt)
    Ppxlib.Pprintast.expression
    expr

let error loc err =
  let msg, hint =
    match err with
    | Invalid_action action ->
        ( "Invalid action.",
          Format.asprintf
            "@[<v 2>Accepted actions are aggregate, aggregate_s, aggregate_f, \
             mark, record, record_f, record_s, reset_block_section, span, \
             span_f, span_s, stamp and stop]@,\
             Found: %s@."
            action )
    | Invalid_payload ->
        ( "Invalid or empty attribute payload.",
          Format.asprintf
            "@[<v 2>Accepted attributes payload are:@,\
             - `[@profiler.aggregate_* <string or ident>]@,\
             - `[@profiler.mark [<list of strings or ident>]]@,\
             - `[@profiler.record_* <string or ident>]@,\
             - `[@profiler.span_* <list of strings or ident>]@." )
    | Invalid_aggregate key ->
        ( "Invalid aggregate.",
          Format.asprintf
            "@[<v 2>A [@profiler.aggregate_*] attribute must be a string or an \
             identifier.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Invalid_custom key ->
        ( "Invalid custom.",
          Format.asprintf
            "@[<v 2>A [@profiler.custom] attribute must be a function \
             application.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Invalid_mark key ->
        ( "Invalid mark.",
          Format.asprintf
            "@[<v 2>A [@profiler.mark] attribute must be a string list.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Invalid_record key ->
        ( "Invalid record.",
          Format.asprintf
            "@[<v 2>A [@profiler.record_*] attribute must be a string or an \
             identifier.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Invalid_span key ->
        ( "Invalid span.",
          Format.asprintf
            "@[<v 2>A [@profiler.span_*] attribute must be a string list.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Invalid_stop key ->
        ( "Invalid stop.",
          Format.asprintf
            "@[<v 2>A [@profiler.stop] should not have an attribute.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
    | Improper_record record ->
        ( "Improper record.",
          Format.asprintf
            "@[<v 2>It looks like you tried to provide some additional options \
             through the mandatory record but no option could be parsed out of \
             it. Possible options are:@,\
             - the verbosity@,\
             - the profiler_module@,\
             Found: { @[<v 2>%a@] }@."
            Format.(
              pp_print_list
                ~pp_sep:(fun ppf () -> Format.fprintf ppf ";@,")
                pp_field)
            record )
    | Improper_field field ->
        ( "Improper field.",
          Format.asprintf
            "@[<v 2>Expecting a field specifying either:@,\
             - the verbosity@,\
             - the profiler_module@,\
             Found: @[<v 0>%a@]@."
            pp_field
            field )
    | Improper_let_binding expr ->
        ( "Improper let binding expression.",
          Format.asprintf
            "@[<v 2>Expecting a let binding expression.@,Found %a@."
            Ppxlib.Pprintast.expression
            expr )
    | Malformed_attribute expr ->
        ( "Malformed attribute.",
          Format.asprintf
            "@[<v 2>Accepted attributes payload are:@,\
             - `[@profiler.mark [<list of strings>]]'@,\
             - `[@profiler.aggregate_* <string or ident>]@,\
             Found %a@.'"
            Ppxlib.Pprintast.expression
            expr )
    | No_verbosity key ->
        ( "Missing mandatory verbosity field",
          Format.asprintf
            "@[<v 2>A [@profiler] call requires a {verbosity} field. Available \
             options are Notice, Info and Debug.@,\
             Found: @[<v 0>%a@]@."
            Key.pp
            key )
  in
  Location.raise_errorf ~loc "profiling_ppx: %s\nHint: %s" msg hint
