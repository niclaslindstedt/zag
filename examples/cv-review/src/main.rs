//! CV Review Pipeline — Example using zag-lib's programmatic API
//!
//! Demonstrates a two-pass AI review pipeline:
//! 1. **Recruiter Screen** — scores a CV against a job description
//! 2. **Hiring Committee** — reviews the recruiter's scores, makes adjustments
//!    with transparent justifications, and produces the final recommendation
//!
//! Usage:
//!   cargo run -p cv-review -- --cv cvs/01_alex_chen.txt --job jobs/senior_backend.txt
//!   cargo run -p cv-review -- --cv-dir cvs/ --job jobs/senior_backend.txt
//!   cargo run -p cv-review -- --cv-dir cvs/ --job jobs/senior_backend.txt --rules scoring_rules.toml

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use zag_lib::builder::AgentBuilder;
use zag_lib::progress::ProgressHandler;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ScoringRules {
    min_overall_score: u8,
    min_experience_years: u8,
    min_relevance_score: u8,
    required_skills: Vec<String>,
    recommendation_pass: Vec<String>,
    weights: ScoringWeights,
}

#[derive(Debug, Deserialize)]
struct ScoringWeights {
    experience: u8,
    skills: u8,
    education: u8,
    communication: u8,
    culture_fit: u8,
}

impl Default for ScoringRules {
    fn default() -> Self {
        Self {
            min_overall_score: 7,
            min_experience_years: 5,
            min_relevance_score: 6,
            required_skills: vec![
                "Rust".into(),
                "distributed systems".into(),
                "AWS".into(),
                "Kafka".into(),
            ],
            recommendation_pass: vec!["strong_yes".into(), "yes".into()],
            weights: ScoringWeights {
                experience: 30,
                skills: 35,
                education: 10,
                communication: 15,
                culture_fit: 10,
            },
        }
    }
}

/// Output from the first LLM pass (Recruiter Screen).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecruiterReview {
    candidate_name: String,
    overall_score: u8,
    category_scores: CategoryScores,
    summary: String,
    strengths: Vec<String>,
    weaknesses: Vec<String>,
    experience_analysis: ExperienceAnalysis,
    skills_match: SkillsMatch,
    recommendation: String,
    suggested_interview_questions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CategoryScores {
    experience: u8,
    skills: u8,
    education: u8,
    communication: u8,
    culture_fit: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperienceAnalysis {
    years_of_experience: u8,
    relevance_score: u8,
    career_progression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillsMatch {
    matched_skills: Vec<String>,
    missing_skills: Vec<String>,
    bonus_skills: Vec<String>,
}

/// Output from the second LLM pass (Hiring Committee).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommitteeReview {
    adjusted_overall_score: u8,
    adjusted_category_scores: CategoryScores,
    score_adjustments: Vec<ScoreAdjustment>,
    committee_notes: String,
    risk_flags: Vec<String>,
    final_recommendation: String,
    interview_focus_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScoreAdjustment {
    category: String,
    original: u8,
    adjusted: u8,
    reason: String,
}

/// Combined result after both passes + programmatic evaluation.
struct CandidateResult {
    cv_file: String,
    recruiter: RecruiterReview,
    committee: CommitteeReview,
    verdict: Verdict,
    weighted_score: f64,
    flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    NeedsReview,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Pass => write!(f, "PASS"),
            Verdict::Fail => write!(f, "FAIL"),
            Verdict::NeedsReview => write!(f, "NEEDS REVIEW"),
        }
    }
}

// ---------------------------------------------------------------------------
// Progress handler — prints status to stderr
// ---------------------------------------------------------------------------

struct ConsoleProgress;

impl ProgressHandler for ConsoleProgress {
    fn on_status(&self, msg: &str) {
        eprintln!("  > {}", msg);
    }
    fn on_success(&self, msg: &str) {
        eprintln!("  \x1b[32m✓\x1b[0m {}", msg);
    }
    fn on_error(&self, msg: &str) {
        eprintln!("  \x1b[31m✗\x1b[0m {}", msg);
    }
    fn on_spinner_start(&self, msg: &str) {
        eprintln!("  ⏳ {}", msg);
    }
    fn on_spinner_finish(&self) {}
}

// ---------------------------------------------------------------------------
// JSON schemas
// ---------------------------------------------------------------------------

fn recruiter_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": [
            "candidate_name", "overall_score", "category_scores", "summary",
            "strengths", "weaknesses", "experience_analysis", "skills_match",
            "recommendation", "suggested_interview_questions"
        ],
        "properties": {
            "candidate_name": { "type": "string" },
            "overall_score": { "type": "integer", "minimum": 1, "maximum": 10 },
            "category_scores": {
                "type": "object",
                "required": ["experience", "skills", "education", "communication", "culture_fit"],
                "properties": {
                    "experience": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "skills": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "education": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "communication": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "culture_fit": { "type": "integer", "minimum": 1, "maximum": 10 }
                },
                "additionalProperties": false
            },
            "summary": { "type": "string" },
            "strengths": { "type": "array", "items": { "type": "string" } },
            "weaknesses": { "type": "array", "items": { "type": "string" } },
            "experience_analysis": {
                "type": "object",
                "required": ["years_of_experience", "relevance_score", "career_progression"],
                "properties": {
                    "years_of_experience": { "type": "integer", "minimum": 0 },
                    "relevance_score": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "career_progression": { "type": "string" }
                },
                "additionalProperties": false
            },
            "skills_match": {
                "type": "object",
                "required": ["matched_skills", "missing_skills", "bonus_skills"],
                "properties": {
                    "matched_skills": { "type": "array", "items": { "type": "string" } },
                    "missing_skills": { "type": "array", "items": { "type": "string" } },
                    "bonus_skills": { "type": "array", "items": { "type": "string" } }
                },
                "additionalProperties": false
            },
            "recommendation": {
                "type": "string",
                "enum": ["strong_yes", "yes", "maybe", "no", "strong_no"]
            },
            "suggested_interview_questions": {
                "type": "array", "items": { "type": "string" }
            }
        },
        "additionalProperties": false
    })
}

fn committee_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": [
            "adjusted_overall_score", "adjusted_category_scores",
            "score_adjustments", "committee_notes", "risk_flags",
            "final_recommendation", "interview_focus_areas"
        ],
        "properties": {
            "adjusted_overall_score": { "type": "integer", "minimum": 1, "maximum": 10 },
            "adjusted_category_scores": {
                "type": "object",
                "required": ["experience", "skills", "education", "communication", "culture_fit"],
                "properties": {
                    "experience": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "skills": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "education": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "communication": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "culture_fit": { "type": "integer", "minimum": 1, "maximum": 10 }
                },
                "additionalProperties": false
            },
            "score_adjustments": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["category", "original", "adjusted", "reason"],
                    "properties": {
                        "category": { "type": "string" },
                        "original": { "type": "integer" },
                        "adjusted": { "type": "integer" },
                        "reason": { "type": "string" }
                    },
                    "additionalProperties": false
                }
            },
            "committee_notes": { "type": "string" },
            "risk_flags": { "type": "array", "items": { "type": "string" } },
            "final_recommendation": {
                "type": "string",
                "enum": ["strong_yes", "yes", "maybe", "no", "strong_no"]
            },
            "interview_focus_areas": {
                "type": "array", "items": { "type": "string" }
            }
        },
        "additionalProperties": false
    })
}

// ---------------------------------------------------------------------------
// LLM calls
// ---------------------------------------------------------------------------

async fn run_recruiter_screen(
    cv: &str,
    job: &str,
    rules: &ScoringRules,
    provider: &str,
    model: &str,
) -> Result<RecruiterReview> {
    let w = &rules.weights;
    let prompt = format!(
        r#"You are an expert technical recruiter performing an initial candidate screen.

Review the following CV against the job description and produce a structured evaluation.

## Scoring Weights
Use these weights to guide your overall assessment:
- Experience: {exp}%
- Skills: {skills}%
- Education: {edu}%
- Communication: {comm}%
- Culture Fit: {culture}%

## Required Skills
The following skills are required for this role. Mark each as matched or missing:
{required}

## Job Description
{job}

## Candidate CV
{cv}

Score each category from 1-10 and provide your assessment."#,
        exp = w.experience,
        skills = w.skills,
        edu = w.education,
        comm = w.communication,
        culture = w.culture_fit,
        required = rules
            .required_skills
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n"),
        job = job,
        cv = cv,
    );

    let output = AgentBuilder::new()
        .provider(provider)
        .model(model)
        .auto_approve(true)
        .system_prompt(
            "You are an expert technical recruiter. \
             Evaluate candidates objectively based on evidence from their CV. \
             Be calibrated: a score of 5 means average, 7 means good, 9+ means exceptional.",
        )
        .json_schema(recruiter_schema())
        .on_progress(Box::new(ConsoleProgress))
        .exec(&prompt)
        .await
        .context("Recruiter screen LLM call failed")?;

    let text = output
        .result
        .as_deref()
        .context("No result from recruiter screen")?;

    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(cleaned).context("Failed to parse recruiter review JSON")
}

async fn run_committee_review(
    cv: &str,
    job: &str,
    recruiter: &RecruiterReview,
    provider: &str,
    model: &str,
) -> Result<CommitteeReview> {
    let recruiter_json =
        serde_json::to_string_pretty(recruiter).context("Failed to serialize recruiter review")?;

    let prompt = format!(
        r#"You are a hiring committee reviewing a recruiter's initial candidate assessment.

Your role is to:
1. Review the recruiter's scores and reasoning
2. Adjust any scores you disagree with, providing clear justification for each change
3. Flag any risks the recruiter may have missed
4. Make a final recommendation

Be rigorous. If you agree with a score, do NOT include it in score_adjustments —
only include categories where you are making a change. If you agree with everything,
score_adjustments should be an empty array.

## Original CV
{cv}

## Job Description
{job}

## Recruiter's Assessment
{recruiter}

Provide your committee review with any score adjustments and final recommendation."#,
        cv = cv,
        job = job,
        recruiter = recruiter_json,
    );

    let output = AgentBuilder::new()
        .provider(provider)
        .model(model)
        .auto_approve(true)
        .system_prompt(
            "You are a senior hiring committee member. \
             You calibrate recruiter scores, catch biases, and ensure hiring quality. \
             Be specific about why you adjust scores. \
             Your adjustments should be transparent and defensible.",
        )
        .json_schema(committee_schema())
        .on_progress(Box::new(ConsoleProgress))
        .exec(&prompt)
        .await
        .context("Committee review LLM call failed")?;

    let text = output
        .result
        .as_deref()
        .context("No result from committee review")?;

    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(cleaned).context("Failed to parse committee review JSON")
}

// ---------------------------------------------------------------------------
// Programmatic scoring & verdict
// ---------------------------------------------------------------------------

fn compute_weighted_score(scores: &CategoryScores, weights: &ScoringWeights) -> f64 {
    let total_weight = weights.experience
        + weights.skills
        + weights.education
        + weights.communication
        + weights.culture_fit;
    if total_weight == 0 {
        return 0.0;
    }
    let weighted = (scores.experience as f64 * weights.experience as f64)
        + (scores.skills as f64 * weights.skills as f64)
        + (scores.education as f64 * weights.education as f64)
        + (scores.communication as f64 * weights.communication as f64)
        + (scores.culture_fit as f64 * weights.culture_fit as f64);
    weighted / total_weight as f64
}

fn evaluate(
    cv_file: &str,
    recruiter: &RecruiterReview,
    committee: &CommitteeReview,
    rules: &ScoringRules,
) -> CandidateResult {
    let mut flags = Vec::new();
    let mut fail_reasons = 0;

    // Use committee's adjusted scores for final evaluation
    let weighted = compute_weighted_score(&committee.adjusted_category_scores, &rules.weights);

    if committee.adjusted_overall_score < rules.min_overall_score {
        flags.push(format!(
            "Overall score {} < minimum {}",
            committee.adjusted_overall_score, rules.min_overall_score
        ));
        fail_reasons += 1;
    }

    if recruiter.experience_analysis.years_of_experience < rules.min_experience_years {
        flags.push(format!(
            "Experience {} years < minimum {} years",
            recruiter.experience_analysis.years_of_experience, rules.min_experience_years
        ));
        fail_reasons += 1;
    }

    if recruiter.experience_analysis.relevance_score < rules.min_relevance_score {
        flags.push(format!(
            "Relevance score {} < minimum {}",
            recruiter.experience_analysis.relevance_score, rules.min_relevance_score
        ));
        fail_reasons += 1;
    }

    // Check required skills coverage
    let matched_lower: Vec<String> = recruiter
        .skills_match
        .matched_skills
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let mut missing_required = Vec::new();
    for skill in &rules.required_skills {
        if !matched_lower
            .iter()
            .any(|m| m.contains(&skill.to_lowercase()))
        {
            missing_required.push(skill.clone());
        }
    }
    if !missing_required.is_empty() {
        flags.push(format!(
            "Missing required skills: {}",
            missing_required.join(", ")
        ));
        if missing_required.len() > 1 {
            fail_reasons += 1;
        }
    }

    // Check recommendation
    let rec = &committee.final_recommendation;
    if !rules.recommendation_pass.contains(rec) {
        flags.push(format!("Recommendation '{}' not in pass list", rec));
        if rec == "no" || rec == "strong_no" {
            fail_reasons += 1;
        }
    }

    // Add committee risk flags
    for flag in &committee.risk_flags {
        flags.push(format!("Risk: {}", flag));
    }

    let verdict = if fail_reasons >= 2 {
        Verdict::Fail
    } else if fail_reasons == 1 || !committee.risk_flags.is_empty() {
        Verdict::NeedsReview
    } else {
        Verdict::Pass
    };

    CandidateResult {
        cv_file: cv_file.to_string(),
        recruiter: recruiter.clone(),
        committee: committee.clone(),
        verdict,
        weighted_score: weighted,
        flags,
    }
}

// ---------------------------------------------------------------------------
// Report printing
// ---------------------------------------------------------------------------

fn score_bar(score: u8) -> String {
    let filled = score as usize;
    let empty = 10 - filled;
    let color = if score >= 8 {
        "\x1b[32m" // green
    } else if score >= 6 {
        "\x1b[33m" // yellow
    } else {
        "\x1b[31m" // red
    };
    format!(
        "{}{}{} {}/10",
        color,
        "█".repeat(filled),
        "\x1b[38;5;240m░\x1b[0m".repeat(empty),
        score
    )
}

fn verdict_colored(verdict: Verdict) -> String {
    match verdict {
        Verdict::Pass => format!("\x1b[1;32m{}\x1b[0m", verdict),
        Verdict::Fail => format!("\x1b[1;31m{}\x1b[0m", verdict),
        Verdict::NeedsReview => format!("\x1b[1;33m{}\x1b[0m", verdict),
    }
}

fn print_report(result: &CandidateResult) {
    let r = &result.recruiter;
    let c = &result.committee;

    println!("\n{}", "═".repeat(72));
    println!("\x1b[1m  {} — {}\x1b[0m", r.candidate_name, result.cv_file);
    println!("{}", "═".repeat(72));

    // Verdict
    println!(
        "\n  Verdict: {}   Weighted Score: {:.1}/10",
        verdict_colored(result.verdict),
        result.weighted_score
    );
    println!(
        "  Overall: {}  Recommendation: \x1b[1m{}\x1b[0m",
        score_bar(c.adjusted_overall_score),
        c.final_recommendation
    );

    // Summary
    println!("\n\x1b[1m  Summary\x1b[0m");
    println!("  {}", r.summary);

    // Category scores (recruiter vs committee)
    println!("\n\x1b[1m  Category Scores\x1b[0m");
    println!(
        "  {:<16} {:>10} {:>10} {:>6}",
        "Category", "Recruiter", "Committee", "Delta"
    );
    println!("  {}", "─".repeat(46));

    let categories = [
        (
            "Experience",
            r.category_scores.experience,
            c.adjusted_category_scores.experience,
        ),
        (
            "Skills",
            r.category_scores.skills,
            c.adjusted_category_scores.skills,
        ),
        (
            "Education",
            r.category_scores.education,
            c.adjusted_category_scores.education,
        ),
        (
            "Communication",
            r.category_scores.communication,
            c.adjusted_category_scores.communication,
        ),
        (
            "Culture Fit",
            r.category_scores.culture_fit,
            c.adjusted_category_scores.culture_fit,
        ),
    ];

    for (name, orig, adj) in &categories {
        let delta = *adj as i16 - *orig as i16;
        let delta_str = if delta > 0 {
            format!("\x1b[32m+{}\x1b[0m", delta)
        } else if delta < 0 {
            format!("\x1b[31m{}\x1b[0m", delta)
        } else {
            "  ".to_string()
        };
        println!("  {:<16} {:>10} {:>10} {:>6}", name, orig, adj, delta_str);
    }

    // Score adjustments from committee
    if !c.score_adjustments.is_empty() {
        println!("\n\x1b[1m  Committee Adjustments\x1b[0m");
        for adj in &c.score_adjustments {
            let arrow = if adj.adjusted > adj.original {
                format!("\x1b[32m{} → {}\x1b[0m", adj.original, adj.adjusted)
            } else {
                format!("\x1b[31m{} → {}\x1b[0m", adj.original, adj.adjusted)
            };
            println!("  • {} ({}): {}", adj.category, arrow, adj.reason);
        }
    }

    // Committee notes
    println!("\n\x1b[1m  Committee Notes\x1b[0m");
    println!("  {}", c.committee_notes);

    // Strengths
    println!("\n\x1b[1m  Strengths\x1b[0m");
    for s in &r.strengths {
        println!("  \x1b[32m+\x1b[0m {}", s);
    }

    // Weaknesses
    println!("\n\x1b[1m  Weaknesses\x1b[0m");
    for w in &r.weaknesses {
        println!("  \x1b[31m-\x1b[0m {}", w);
    }

    // Experience
    println!("\n\x1b[1m  Experience Analysis\x1b[0m");
    println!(
        "  Years: {}  |  Relevance: {}",
        r.experience_analysis.years_of_experience,
        score_bar(r.experience_analysis.relevance_score)
    );
    println!("  {}", r.experience_analysis.career_progression);

    // Skills match
    println!("\n\x1b[1m  Skills Match\x1b[0m");
    if !r.skills_match.matched_skills.is_empty() {
        println!(
            "  \x1b[32mMatched:\x1b[0m {}",
            r.skills_match.matched_skills.join(", ")
        );
    }
    if !r.skills_match.missing_skills.is_empty() {
        println!(
            "  \x1b[31mMissing:\x1b[0m {}",
            r.skills_match.missing_skills.join(", ")
        );
    }
    if !r.skills_match.bonus_skills.is_empty() {
        println!(
            "  \x1b[36mBonus:\x1b[0m  {}",
            r.skills_match.bonus_skills.join(", ")
        );
    }

    // Risk flags
    if !c.risk_flags.is_empty() {
        println!("\n\x1b[1m  Risk Flags\x1b[0m");
        for flag in &c.risk_flags {
            println!("  \x1b[33m⚠\x1b[0m {}", flag);
        }
    }

    // Interview focus areas (only if not failed)
    if result.verdict != Verdict::Fail && !c.interview_focus_areas.is_empty() {
        println!("\n\x1b[1m  Interview Focus Areas\x1b[0m");
        for (i, area) in c.interview_focus_areas.iter().enumerate() {
            println!("  {}. {}", i + 1, area);
        }
    }

    // Suggested interview questions (only if passed)
    if result.verdict == Verdict::Pass && !r.suggested_interview_questions.is_empty() {
        println!("\n\x1b[1m  Suggested Interview Questions\x1b[0m");
        for (i, q) in r.suggested_interview_questions.iter().enumerate() {
            println!("  {}. {}", i + 1, q);
        }
    }

    // Evaluation flags
    if !result.flags.is_empty() {
        println!("\n\x1b[1m  Evaluation Flags\x1b[0m");
        for flag in &result.flags {
            let icon = if flag.starts_with("Risk:") {
                "\x1b[33m⚠\x1b[0m"
            } else {
                "\x1b[31m✗\x1b[0m"
            };
            println!("  {} {}", icon, flag);
        }
    }

    println!();
}

fn print_summary_table(results: &[CandidateResult]) {
    println!("\n{}", "═".repeat(96));
    println!("\x1b[1m  CANDIDATE RANKING SUMMARY\x1b[0m");
    println!("{}", "═".repeat(96));
    println!(
        "  {:<4} {:<22} {:>8} {:>10} {:>8} {:>14} {:>12}",
        "Rank", "Candidate", "Score", "Weighted", "Exp(yr)", "Recommendation", "Verdict"
    );
    println!("  {}", "─".repeat(90));

    // Sort by weighted score descending
    let mut sorted: Vec<(usize, &CandidateResult)> = results.iter().enumerate().collect();
    sorted.sort_by(|a, b| {
        b.1.weighted_score
            .partial_cmp(&a.1.weighted_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (rank, (_, result)) in sorted.iter().enumerate() {
        let verdict_str = match result.verdict {
            Verdict::Pass => "\x1b[32mPASS\x1b[0m",
            Verdict::Fail => "\x1b[31mFAIL\x1b[0m",
            Verdict::NeedsReview => "\x1b[33mREVIEW\x1b[0m",
        };
        println!(
            "  {:<4} {:<22} {:>5}/10 {:>9.1} {:>5} {:>14} {:>12}",
            rank + 1,
            result.recruiter.candidate_name,
            result.committee.adjusted_overall_score,
            result.weighted_score,
            result.recruiter.experience_analysis.years_of_experience,
            result.committee.final_recommendation,
            verdict_str,
        );
    }

    let pass_count = results
        .iter()
        .filter(|r| r.verdict == Verdict::Pass)
        .count();
    let review_count = results
        .iter()
        .filter(|r| r.verdict == Verdict::NeedsReview)
        .count();
    let fail_count = results
        .iter()
        .filter(|r| r.verdict == Verdict::Fail)
        .count();

    println!("  {}", "─".repeat(90));
    println!(
        "  Total: {}  |  \x1b[32mPass: {}\x1b[0m  |  \x1b[33mReview: {}\x1b[0m  |  \x1b[31mFail: {}\x1b[0m",
        results.len(),
        pass_count,
        review_count,
        fail_count
    );
    println!();
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

struct Args {
    cv: Option<PathBuf>,
    cv_dir: Option<PathBuf>,
    job: PathBuf,
    rules: Option<PathBuf>,
    provider: String,
    model: String,
}

fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let mut cv = None;
    let mut cv_dir = None;
    let mut job = None;
    let mut rules = None;
    let mut provider = "claude".to_string();
    let mut model = "sonnet".to_string();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cv" => cv = Some(PathBuf::from(args.next().context("--cv requires a path")?)),
            "--cv-dir" => {
                cv_dir = Some(PathBuf::from(
                    args.next().context("--cv-dir requires a path")?,
                ))
            }
            "--job" => job = Some(PathBuf::from(args.next().context("--job requires a path")?)),
            "--rules" => {
                rules = Some(PathBuf::from(
                    args.next().context("--rules requires a path")?,
                ))
            }
            "--provider" | "-p" => provider = args.next().context("--provider requires a value")?,
            "--model" | "-m" => model = args.next().context("--model requires a value")?,
            "--help" | "-h" => {
                eprintln!(
                    "Usage: cv-review [OPTIONS]\n\n\
                     Options:\n  \
                       --cv <PATH>        Single CV file to review\n  \
                       --cv-dir <PATH>    Directory of CV files (batch mode)\n  \
                       --job <PATH>       Job description file [default: jobs/senior_backend.txt]\n  \
                       --rules <PATH>     Scoring rules TOML file [default: built-in rules]\n  \
                       --provider, -p     LLM provider [default: claude]\n  \
                       --model, -m        Model name [default: sonnet]\n  \
                       --help, -h         Show this help\n\n\
                     Examples:\n  \
                       cv-review --cv cvs/01_alex_chen.txt --job jobs/senior_backend.txt\n  \
                       cv-review --cv-dir cvs/ --job jobs/senior_backend.txt\n  \
                       cv-review --cv-dir cvs/ --job jobs/fullstack_lead.txt --rules scoring_rules.toml"
                );
                std::process::exit(0);
            }
            other => bail!("Unknown argument: {}. Use --help for usage.", other),
        }
    }

    if cv.is_none() && cv_dir.is_none() {
        bail!("Provide either --cv <path> or --cv-dir <path>. Use --help for usage.");
    }

    Ok(Args {
        cv,
        cv_dir,
        job: job.unwrap_or_else(|| PathBuf::from("jobs/senior_backend.txt")),
        rules,
        provider,
        model,
    })
}

fn load_rules(path: Option<&PathBuf>) -> Result<ScoringRules> {
    match path {
        Some(p) => {
            let content = std::fs::read_to_string(p)
                .with_context(|| format!("Failed to read rules file: {}", p.display()))?;
            toml::from_str(&content).context("Failed to parse scoring rules TOML")
        }
        None => Ok(ScoringRules::default()),
    }
}

fn collect_cv_paths(args: &Args) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    if let Some(ref cv) = args.cv {
        paths.push(cv.clone());
    }

    if let Some(ref dir) = args.cv_dir {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
            .collect();
        entries.sort();
        paths.extend(entries);
    }

    if paths.is_empty() {
        bail!("No CV files found");
    }

    Ok(paths)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let rules = load_rules(args.rules.as_ref())?;
    let job = std::fs::read_to_string(&args.job)
        .with_context(|| format!("Failed to read job file: {}", args.job.display()))?;
    let cv_paths = collect_cv_paths(&args)?;

    eprintln!(
        "\x1b[1mCV Review Pipeline\x1b[0m — {} candidate(s), provider={}, model={}",
        cv_paths.len(),
        args.provider,
        args.model
    );
    eprintln!(
        "Job: {}\n",
        args.job.file_name().unwrap_or_default().to_string_lossy()
    );

    let mut results = Vec::new();

    for (i, cv_path) in cv_paths.iter().enumerate() {
        let cv_name = cv_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        eprintln!(
            "\x1b[1m[{}/{}] Processing: {}\x1b[0m",
            i + 1,
            cv_paths.len(),
            cv_name
        );

        let cv_text = std::fs::read_to_string(cv_path)
            .with_context(|| format!("Failed to read CV: {}", cv_path.display()))?;

        // Pass 1: Recruiter Screen
        eprintln!("  Pass 1: Recruiter Screen");
        let recruiter =
            run_recruiter_screen(&cv_text, &job, &rules, &args.provider, &args.model).await?;
        eprintln!(
            "  \x1b[32m✓\x1b[0m Score: {}/10, Recommendation: {}",
            recruiter.overall_score, recruiter.recommendation
        );

        // Pass 2: Hiring Committee
        eprintln!("  Pass 2: Hiring Committee Review");
        let committee =
            run_committee_review(&cv_text, &job, &recruiter, &args.provider, &args.model).await?;
        eprintln!(
            "  \x1b[32m✓\x1b[0m Adjusted: {}/10, Final: {}",
            committee.adjusted_overall_score, committee.final_recommendation
        );

        // Programmatic evaluation
        let result = evaluate(&cv_name, &recruiter, &committee, &rules);
        eprintln!("  → {}\n", verdict_colored(result.verdict));

        results.push(result);
    }

    // Print individual reports
    for result in &results {
        print_report(result);
    }

    // Print summary table in batch mode
    if results.len() > 1 {
        print_summary_table(&results);
    }

    Ok(())
}
