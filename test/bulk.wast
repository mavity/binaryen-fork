(module
  (memory 1)
  (data (i32.const 0) "hello")
  (func (export "fill") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.fill
  )
  (func (export "copy") (param i32 i32 i32)
    local.get 0
    local.get 1
    local.get 2
    memory.copy
  )
)
