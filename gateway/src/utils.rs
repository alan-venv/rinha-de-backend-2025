pub const FLAG_TIMEOUT_TRIGGER_IN_MILLISECONDS: u128 = 200;
pub const WORKER_COUNT: usize = 2; // mais que isso eu perco o bonus de p99.
pub const WORKER_JOBS_COUNT: usize = 1000;

// Configurações ideais para até MAX_REQUESTS=1200
// Mais que isso tem que aceitar a perda do bonus de p99 e subir o JOBS_COUNT para previnir o lag de pagamentos.
