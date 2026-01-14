(module
  ;; Function with identity operations for SimplifyIdentity
  (func $compute (param $x i32) (param $y i32) (result i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    (local $result i32)
    
    ;; Identity operations that should be optimized
    (local.set $a
      (i32.add (local.get $x) (i32.const 0)))  ;; x + 0 -> x
    
    (local.set $b
      (i32.mul (local.get $y) (i32.const 1)))  ;; y * 1 -> y
    
    (local.set $c
      (i32.add (local.get $a) (local.get $b)))
    
    ;; Some real computation
    (local.set $result
      (i32.mul (local.get $c) (i32.const 2)))
    
    (local.set $result
      (i32.add (local.get $result) (i32.const 10)))
    
    (local.get $result))
  
  ;; Function with unreachable code for DCE
  (func $check_bounds (param $value i32) (param $max i32) (result i32)
    (local $temp i32)
    
    (if (i32.lt_s (local.get $value) (i32.const 0))
      (then
        (return (i32.const -1))
        ;; Dead code after return - DCE should remove this
        (local.set $temp (i32.mul (local.get $value) (i32.const 2)))
        (local.set $temp (i32.add (local.get $temp) (i32.const 100)))
        (drop (local.get $temp))))
    
    (if (i32.gt_s (local.get $value) (local.get $max))
      (then
        (return (i32.const -2))
        ;; More dead code
        (local.set $temp (i32.div_s (local.get $value) (i32.const 2)))
        (drop (local.get $temp))))
    
    (local.get $value))
  
  ;; Function combining both optimization opportunities
  (func $process (param $element i32) (param $index i32) (result i32)
    (local $adjusted i32)
    (local $result i32)
    
    ;; Multiple identity operations
    (local.set $adjusted
      (i32.add (local.get $element) (i32.const 0)))  ;; element + 0
    
    (local.set $adjusted
      (i32.mul (local.get $adjusted) (i32.const 1)))  ;; adjusted * 1
    
    ;; Bounds check with dead code
    (if (i32.lt_s (local.get $adjusted) (i32.const 0))
      (then
        (return (i32.const 0))
        ;; Dead code after return
        (local.set $adjusted (i32.add (local.get $adjusted) (i32.const 1000)))))
    
    ;; Apply transformation with more identity ops
    (local.set $result
      (i32.add (local.get $adjusted) (local.get $index)))
    
    (local.set $result
      (i32.mul (local.get $result) (i32.const 1)))  ;; Another identity
    
    (local.get $result))
  
  ;; Exported functions
  (export "compute" (func $compute))
  (export "check_bounds" (func $check_bounds))
  (export "process" (func $process)))
