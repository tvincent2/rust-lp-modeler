use std;
use variables::*;
use variables::LpExpression::*;
use std::rc::Rc;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

//use variables::LpExpression::{AddExpr, MulExpr};
use std::ops::{AddAssign};

//use std::collections::HashMap;

/// Enum helping to specify the objective function of the linear problem.
///
/// # Examples:
///
/// ```
/// use lp_modeler::problem::{LpObjective, LpProblem};
///
/// let mut problem = LpProblem::new("One Problem", LpObjective::Maximize);
/// ```
#[derive(Debug, PartialEq)]
pub enum LpObjective {
    Minimize,
    Maximize
}

pub trait Problem {
    fn add_objective_expression(&mut self, expr: &LpExpression);
    fn add_constraints(&mut self, expr: &LpConstraint);
}

/// Structure used for creating the model and solving a linear problem.
///
/// # Examples:
///
/// ```
/// use lp_modeler::problem::{LpObjective, Problem, LpProblem};
/// use lp_modeler::operations::{LpOperations};
/// use lp_modeler::variables::LpInteger;
/// use lp_modeler::solvers::{SolverTrait, CbcSolver};
///
/// let ref a = LpInteger::new("a");
/// let ref b = LpInteger::new("b");
/// let ref c = LpInteger::new("c");
///
/// let mut problem = LpProblem::new("One Problem", LpObjective::Maximize);
/// problem += 10.0 * a + 20.0 * b;
///
/// problem += (500*a + 1200*b + 1500*c).le(10000);
/// problem += (a + b*2 + c).le(10);
/// problem += (a).le(b);
///
/// let solver = CbcSolver::new();
///
/// match solver.run(&problem) {
/// Ok((status, res)) => {
///     println!("Status {:?}", status);
///         for (name, value) in res.iter() {
///             println!("value of {} = {}", name, value);
///         }
///     },
///     Err(msg) => println!("{}", msg),
/// }
/// ```
#[derive(Debug)]
pub struct LpProblem {
    name: &'static str,
    objective_type: LpObjective,
    obj_expr: Option<LpExpression>,
    constraints: Vec<LpConstraint>

}

impl LpProblem {
    /// Create a new problem
    pub fn new(name: &'static str, objective: LpObjective) -> LpProblem {
        LpProblem { name: name, objective_type: objective, obj_expr: None, constraints: Vec::new() }
    }

    // TODO: Call once and pass into parameter
    // TODO: Check variables on the objective function
    fn variables(&self) -> HashMap<String, &LpExpression> {
        fn var<'a>(expr: &'a LpExpression, lst: &mut Vec<(String, &'a LpExpression)>) {
            match expr {
                &ConsBin(LpBinary { ref name, .. }) |
                &ConsInt(LpInteger { ref name, .. }) |
                &ConsCont(LpContinuous { ref name, .. })
                => { lst.push((name.clone(), expr)); },

                &MulExpr(_, ref e) => { var(&*e, lst); },
                &AddExpr(ref e1, ref e2) | &SubExpr(ref e1, ref e2) => {
                    var(&*e1, lst);
                    var(&*e2, lst);
                },
                _ => ()
            }
        }

        let mut lst: Vec<_> = Vec::new();
        for e in &self.constraints {
            var(&e.0, &mut lst);
        }
        lst.iter().map(|&(ref n, ref x)| (n.clone(), *x)).collect::<HashMap<String, &LpExpression>>()
    }

}

pub trait LpFileFormat {
    fn to_lp_file_format(&self) -> String;
    fn write_lp(&self, file_model: &str) -> std::io::Result<()>  {
        let mut buffer = try!(File::create(file_model));
        try!(buffer.write(self.to_lp_file_format().as_bytes()));
        Ok(())
    }
}

impl LpFileFormat for LpProblem {
    fn to_lp_file_format(&self) -> String {

        let objective_string = || {
            if let Some(ref expr) = self.obj_expr {
                format!("obj: {}", expr.to_lp_file_format())
            } else {
                String::new()
            }
        };

        let constraints_string = || {
            let mut res = String::new();
            let mut cstrs = self.constraints.iter();
            let mut index = 1;
            while let Some(ref constraint) = cstrs.next() {
                res.push_str("  c");
                res.push_str(&index.to_string());
                res.push_str(": ");
                index += 1;

                res.push_str(&constraint.to_lp_file_format());

                res.push_str("\n");
            }
            res
        };

        let bounds_string = || {
            let mut res = String::new();
            for (_, v) in self.variables() {
                match v {
                    &ConsInt(LpInteger { ref name, lower_bound, upper_bound })
                    | &ConsCont(LpContinuous { ref name, lower_bound, upper_bound }) => {
                        if let Some(l) = lower_bound {
                            res.push_str("  ");
                            res.push_str(&l.to_string());
                            res.push_str(" <= ");
                            res.push_str(&name);
                            if let Some(u) = upper_bound {
                                res.push_str(" <= ");
                                res.push_str(&u.to_string());
                            }
                            res.push_str("\n");
                        } else if let Some(u) = upper_bound {
                            res.push_str("  ");
                            res.push_str(&name);
                            res.push_str(" <= ");
                            res.push_str(&u.to_string());
                            res.push_str("\n");
                        } else {
                            match v {
                                &ConsCont(LpContinuous { .. }) => {
                                    res.push_str("  ");
                                    res.push_str(&name);
                                    res.push_str(" free\n");
                                }, // TODO: IntegerVar => -INF to INF
                                _ => ()
                            }
                        }
                    },
                    _ => (),
                }
            }
            res
        };

        let integers_string = || {
            let mut res = String::new();
            for (_, v) in self.variables() {
                match v {
                    &ConsInt(LpInteger { ref name, .. }) => {
                        res.push_str(name);
                        res.push_str(" ");
                    },
                    _ => (),
                }
            }
            res
        };

        let binaries_string = || {
            let mut res = String::new();
            for (_, v) in self.variables() {
                match v {
                    &ConsBin(LpBinary { ref name }) => {
                        res.push_str(name);
                        res.push_str(" ");
                    },
                    _ => (),
                }
            }
            res
        };


        let mut buffer = String::new();

        buffer.push_str("\\ ");
        buffer.push_str(&self.name);
        buffer.push_str("\n\n");

        // Write objectives
        match self.objective_type {
            LpObjective::Maximize => { buffer.push_str("Maximize\n  "); },
            LpObjective::Minimize => { buffer.push_str("Minimize\n  "); },
        }
        let obj_str = objective_string();
        buffer.push_str(&obj_str);

        // Write constraints
        let cstr_str = constraints_string();
        if cstr_str.len() > 0 {
            buffer.push_str("\n\nSubject To\n");
            buffer.push_str(&cstr_str);
        }

        // Write bounds for Integer and Continuous
        let bound_str = bounds_string();
        if bound_str.len() > 0 {
            buffer.push_str("\nBounds\n");
            buffer.push_str(&bound_str);
        }

        // Write Integer vars
        let generals_str = integers_string();
        if generals_str.len() > 0 {
            buffer.push_str("\nGenerals\n  ");
            buffer.push_str(&generals_str);
            buffer.push_str("\n");
        }

        // Write Binaries vars
        let binaries_str = binaries_string();
        if binaries_str.len() > 0 {
            buffer.push_str("\nBinary\n  ");
            buffer.push_str(&binaries_str);
            buffer.push_str("\n");
        }

        buffer.push_str("\nEnd");

        buffer
    }
}

impl Problem for LpProblem {

    fn add_objective_expression(&mut self, expr: &LpExpression) {
        if let Some(e) = self.obj_expr.clone() {
            let (_, simpl_expr) = split_constant_and_expr(&simplify(&AddExpr(Rc::new(expr.clone()), Rc::new(e.clone()))));
            self.obj_expr = Some(simpl_expr);
        } else {
            let (_, simpl_expr) = split_constant_and_expr(&simplify(expr));
            self.obj_expr = Some(simpl_expr);
        }
    }

    fn add_constraints(&mut self, expr: &LpConstraint) {
        self.constraints.push(expr.clone());
    }

}

/*
/// Add constraints
impl AddAssign<LpConstraint> for LpProblem {
    fn add_assign(&mut self, _rhs: LpConstraint) {
        self.add_constraints(&_rhs);
    }
}

/// Add an expression as an objective function
impl<T> AddAssign<T> for LpProblem where T: Into<LpExpression>{
    fn add_assign(&mut self, _rhs: T) {
        self.add_objective_expression(&_rhs.into());
    }
}
*/

macro_rules! impl_addassign_for_generic_problem {
    ($problem: ty) => {
        /// Add constraints
        impl AddAssign<LpConstraint> for $problem {
            fn add_assign(&mut self, _rhs: LpConstraint) {
                self.add_constraints(&_rhs);
            }
        }
        /// Add an expression as an objective function
        impl<T> AddAssign<T> for $problem where T: Into<LpExpression>{
            fn add_assign(&mut self, _rhs: T) {
                self.add_objective_expression(&_rhs.into());
            }
        }
    }
}
impl_addassign_for_generic_problem!(LpProblem);
