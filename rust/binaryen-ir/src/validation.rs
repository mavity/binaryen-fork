use crate::expression::{Expression, ExpressionKind};
use crate::module::{Function, Module};
use crate::visitor::ReadOnlyVisitor;
use binaryen_core::Type;

pub struct Validator<'a, 'm> {
    module: &'m Module<'a>,
    current_function: Option<&'m Function<'a>>,
    valid: bool,
    errors: Vec<String>,
}

impl<'a, 'm> Validator<'a, 'm> {
    pub fn new(module: &'m Module<'a>) -> Self {
        Self {
            module,
            current_function: None,
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn validate(mut self) -> (bool, Vec<String>) {
        // Validate each function
        for func in &self.module.functions {
            self.current_function = Some(func);
            let context = format!("Function '{}': ", func.name);

            // Check body if present
            if let Some(body) = &func.body {
                self.visit(body);

                // Check return type
                if body.type_ != func.results {
                    // Simple check: Allow Unreachable
                    if body.type_ != Type::UNREACHABLE {
                        self.fail(&format!(
                            "{}Result mismatch. Expected {:?}, got {:?}",
                            context, func.results, body.type_
                        ));
                    }
                }
            }
        }
        (self.valid, self.errors)
    }

    fn fail(&mut self, msg: &str) {
        self.valid = false;
        self.errors.push(msg.to_string());
    }
}

impl<'a, 'm> ReadOnlyVisitor<'a> for Validator<'a, 'm> {
    fn visit_expression(&mut self, expr: &Expression<'a>) {
        match &expr.kind {
            ExpressionKind::Binary { op, left, right } => {
                if left.type_ != right.type_ {
                    if left.type_ != Type::UNREACHABLE && right.type_ != Type::UNREACHABLE {
                        self.fail(&format!(
                            "Binary op {:?} operands type mismatch: {:?} vs {:?}",
                            op, left.type_, right.type_
                        ));
                    }
                }
            }
            ExpressionKind::LocalGet { index: _ } => {
                // TODO: Validate index bounds (need Type tuple support)
            }
            ExpressionKind::Call {
                target, operands, ..
            } => {
                if let Some(func) = self.module.get_function(target) {
                    if operands.len() != 0 && !func.params.is_basic() {
                        // TODO: Check tuple params
                    } else if operands.len() == 1 && func.params.is_basic() {
                        // Check single param
                        let op_type = operands[0].type_;
                        if op_type != func.params && op_type != Type::UNREACHABLE {
                            self.fail(&format!("Call to {} param mismatch", target));
                        }
                    }
                } else {
                    self.fail(&format!("Call target not found: {}", target));
                }
            }
            _ => {}
        }
    }
}
