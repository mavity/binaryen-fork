(module
  ;; Very simple function with one param, identity operations
  (func $identity_test (param $x i32) (result i32)
    (local $temp i32)
    
    ;; Identity operation: x + 0 -> x
    (local.set $temp
      (i32.add (local.get $x) (i32.const 0)))
    
    ;; Another identity: temp * 1 -> temp  
    (i32.mul (local.get $temp) (i32.const 1)))
  
  (export "identity_test" (func $identity_test)))
