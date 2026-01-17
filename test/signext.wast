(module
  (func (export "extend8_s") (param i32) (result i32)
    local.get 0
    i32.extend8_s
  )
  (func (export "extend16_s") (param i32) (result i32)
    local.get 0
    i32.extend16_s
  )
  (func (export "extend8_s_i64") (param i64) (result i64)
    local.get 0
    i64.extend8_s
  )
  (func (export "extend16_s_i64") (param i64) (result i64)
    local.get 0
    i64.extend16_s
  )
  (func (export "extend32_s_i64") (param i64) (result i64)
    local.get 0
    i64.extend32_s
  )
)
