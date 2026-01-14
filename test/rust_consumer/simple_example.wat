(module
  ;; Simple function with identity operations for SimplifyIdentity
  (func $compute (param $x i32) (param $y i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    
    ;; Identity operations that should be optimized
    (local.set $a
      (i32.add (local.get $x) (i32.const 0)))  ;; x + 0 -> x
    
    (local.set $b
      (i32.mul (local.get $y) (i32.const 1)))  ;; y * 1 -> y
    
    (local.set $c
      (i32.add (local.get $a) (local.get $b)))
    
    (local.get $c))
  
  (export "compute" (func $compute)))
