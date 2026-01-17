(module
  (memory 1)
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.atomic.rmw.add
  )
  (func (export "wait") (param i32 i32 i64) (result i32)
    local.get 0
    local.get 1
    local.get 2
    memory.atomic.wait32
  )
)
