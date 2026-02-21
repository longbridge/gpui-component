open Printf

(* A variant type representing shapes *)
type shape =
  | Circle of float
  | Rectangle of float * float
  | Triangle of { base : float; height : float }

(* A record type for configuration *)
type config = {
  name : string;
  debug : bool;
  retries : int;
}

(* A polymorphic option helper *)
let unwrap_or (default : 'a) (opt : 'a option) : 'a =
  match opt with
  | Some x -> x
  | None -> default

(* Compute the area of a shape using pattern matching *)
let area (s : shape) : float =
  match s with
  | Circle r -> Float.pi *. r *. r
  | Rectangle (w, h) -> w *. h
  | Triangle { base; height } -> 0.5 *. base *. height

(* A recursive function *)
let rec factorial (n : int) : int =
  if n <= 1 then 1
  else n * factorial (n - 1)

(* Higher-order function with labeled and optional arguments *)
let greet ~greeting ?(punctuation = "!") name =
  sprintf "%s, %s%s" greeting name punctuation

(* Module with a signature *)
module type Printable = sig
  type t
  val to_string : t -> string
end

module ShapePrinter : Printable with type t = shape = struct
  type t = shape

  let to_string = function
    | Circle r -> sprintf "Circle(r=%.2f)" r
    | Rectangle (w, h) -> sprintf "Rectangle(%.2f x %.2f)" w h
    | Triangle { base; height } ->
      sprintf "Triangle(base=%.2f, height=%.2f)" base height
end

(* Exception handling *)
exception Invalid_input of string

let safe_divide a b =
  if b = 0.0 then raise (Invalid_input "division by zero")
  else a /. b

(* List processing with the pipe operator *)
let process_names names =
  names
  |> List.filter (fun name -> String.length name > 0)
  |> List.map String.uppercase_ascii
  |> List.sort String.compare

(* For loop and mutable reference *)
let sum_range (start : int) (stop : int) : int =
  let total = ref 0 in
  for i = start to stop do
    total := !total + i
  done;
  !total

(* Async-like computation with result type *)
let try_parse (s : string) : (int, string) result =
  try Ok (int_of_string s)
  with Failure msg -> Error msg

(* Entry point *)
let () =
  let shapes = [Circle 3.0; Rectangle (4.0, 5.0); Triangle { base = 6.0; height = 3.0 }] in
  List.iter (fun s ->
    let desc = ShapePrinter.to_string s in
    printf "%s -> area = %.2f\n" desc (area s)
  ) shapes;

  printf "5! = %d\n" (factorial 5);
  printf "%s\n" (greet ~greeting:"Hello" "OCaml");
  printf "sum(1..10) = %d\n" (sum_range 1 10);

  let names = process_names ["alice"; ""; "bob"; "charlie"] in
  List.iter (printf "  %s\n") names;

  match try_parse "42" with
  | Ok n -> printf "Parsed: %d\n" n
  | Error e -> printf "Error: %s\n" e
