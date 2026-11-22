
use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program]
pub mod bella_aura_cosmetics {
    use super::*;

    // Inicializa el administrador principal del programa.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.admin_config;
        config.authority = ctx.accounts.authority.key();
        config.bump = ctx.bumps.admin_config;

        emit!(ProgramInitialized {
            admin: config.authority
        });

        Ok(())
    }

    // Crea un visitante, trabajador o proveedor.
    // Cada categoría tiene su propio contador de IDs.
    pub fn create_record(
        ctx: Context<CreateRecord>,
        category: RecordCategory,
        name: String,
        contact_wallet: Pubkey,
    ) -> Result<()> {
        require!(
            !name.trim().is_empty(),
            BellaAuraError::EmptyName
        );

        require!(
            name.len() <= 64,
            BellaAuraError::NameTooLong
        );

        let counter = &mut ctx.accounts.counter;

        counter.next_id = counter
            .next_id
            .checked_add(1)
            .ok_or(BellaAuraError::CounterOverflow)?;

        let record = &mut ctx.accounts.record;

        record.id = counter.next_id;
        record.category = category;
        record.name = name;
        record.contact_wallet = contact_wallet;
        record.created_by = ctx.accounts.authority.key();
        record.active = true;
        record.activity_count = 0;
        record.bump = ctx.bumps.record;

        emit!(RecordCreated {
            id: record.id,
            category,
            contact_wallet,
            created_by: record.created_by
        });

        Ok(())
    }

    // Modifica un registro existente.
    // El ID y la categoría no cambian.
    pub fn update_record(
        ctx: Context<UpdateRecord>,
        new_name: String,
        new_contact_wallet: Pubkey,
    ) -> Result<()> {
        require!(
            !new_name.trim().is_empty(),
            BellaAuraError::EmptyName
        );

        require!(
            new_name.len() <= 64,
            BellaAuraError::NameTooLong
        );

        let record = &mut ctx.accounts.record;

        require!(
            record.active,
            BellaAuraError::RecordInactive
        );

        record.name = new_name;
        record.contact_wallet = new_contact_wallet;

        emit!(RecordUpdated {
            id: record.id,
            category: record.category
        });

        Ok(())
    }

    // Registra una actividad asociada a una cuenta.
    // El contador impide la baja mientras haya actividad.
    pub fn add_activity(
        ctx: Context<AddActivity>,
        note: String,
    ) -> Result<()> {
        require!(
            !note.trim().is_empty(),
            BellaAuraError::EmptyNote
        );

        require!(
            note.len() <= 120,
            BellaAuraError::NoteTooLong
        );

        let record = &mut ctx.accounts.record;

        require!(
            record.active,
            BellaAuraError::RecordInactive
        );

        record.activity_count = record
            .activity_count
            .checked_add(1)
            .ok_or(BellaAuraError::CounterOverflow)?;

        emit!(ActivityAdded {
            id: record.id,
            category: record.category,
            activity_count: record.activity_count
        });

        Ok(())
    }

    // Da de baja un registro únicamente si no tiene actividad.
    pub fn deactivate_record(
        ctx: Context<DeactivateRecord>,
    ) -> Result<()> {
        let record = &mut ctx.accounts.record;

        require!(
            record.active,
            BellaAuraError::RecordInactive
        );

        require!(
            record.activity_count == 0,
            BellaAuraError::RecordHasActivity
        );

        record.active = false;

        emit!(RecordDeactivated {
            id: record.id,
            category: record.category
        });

        Ok(())
    }
}

// ======================================================
// CATEGORÍAS DE BELLA AURA COSMETICS
// ======================================================

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug
)]
pub enum RecordCategory {
    Visitor,
    Worker,
    Supplier,
}

// ======================================================
// CUENTAS
// ======================================================

// Configuración principal y autoridad administrativa.
#[account]
pub struct AdminConfig {
    pub authority: Pubkey,
    pub bump: u8,
}

impl AdminConfig {
    pub const SPACE: usize = 8 + 32 + 1;
}

// Contador independiente por categoría.
#[account]
pub struct Counter {
    pub next_id: u64,
}

impl Counter {
    pub const SPACE: usize = 8 + 8;
}

// Registro de visitante, trabajador o proveedor.
#[account]
pub struct Record {
    pub id: u64,
    pub category: RecordCategory,
    pub name: String,
    pub contact_wallet: Pubkey,
    pub created_by: Pubkey,
    pub active: bool,
    pub activity_count: u64,
    pub bump: u8,
}

impl Record {
    pub const MAX_NAME_LENGTH: usize = 64;

    pub const SPACE: usize =
        8 +                  // discriminator
        8 +                  // id
        1 +                  // category
        4 + MAX_NAME_LENGTH + // name
        32 +                 // contact wallet
        32 +                 // created by
        1 +                  // active
        8 +                  // activity count
        1;                    // bump
}

// ======================================================
// INSTRUCCIÓN: INITIALIZE
// ======================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = AdminConfig::SPACE,
        seeds = [b"admin"],
        bump
    )]
    pub admin_config: Account<'info, AdminConfig>,

    pub system_program: Program<'info, System>,
}

// ======================================================
// INSTRUCCIÓN: CREATE RECORD
// ======================================================

#[derive(Accounts)]
#[instruction(category: RecordCategory)]
pub struct CreateRecord<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"admin"],
        bump = admin_config.bump,
        has_one = authority
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(
        init_if_needed,
        payer = authority,
        space = Counter::SPACE,
        seeds = [
            b"counter",
            category_seed(&category)
        ],
        bump
    )]
    pub counter: Account<'info, Counter>,

    #[account(
        init,
        payer = authority,
        space = Record::SPACE,
        seeds = [
            b"record",
            category_seed(&category),
            &(counter.next_id.checked_add(1)
                .ok_or(BellaAuraError::CounterOverflow)?)
                .to_le_bytes()
        ],
        bump
    )]
    pub record: Account<'info, Record>,

    pub system_program: Program<'info, System>,
}

// ======================================================
// INSTRUCCIÓN: UPDATE RECORD
// ======================================================

#[derive(Accounts)]
pub struct UpdateRecord<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"admin"],
        bump = admin_config.bump,
        has_one = authority
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(mut)]
    pub record: Account<'info, Record>,
}

// ======================================================
// INSTRUCCIÓN: ADD ACTIVITY
// ======================================================

#[derive(Accounts)]
pub struct AddActivity<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"admin"],
        bump = admin_config.bump,
        has_one = authority
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(mut)]
    pub record: Account<'info, Record>,
}

// ======================================================
// INSTRUCCIÓN: DEACTIVATE RECORD
// ======================================================

#[derive(Accounts)]
pub struct DeactivateRecord<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [b"admin"],
        bump = admin_config.bump,
        has_one = authority
    )]
    pub admin_config: Account<'info, AdminConfig>,

    #[account(mut)]
    pub record: Account<'info, Record>,
}

// ======================================================
// SEMILLAS PDA
// ======================================================

pub fn category_seed(category: &RecordCategory) -> &'static [u8] {
    match category {
        RecordCategory::Visitor => b"visitor",
        RecordCategory::Worker => b"worker",
        RecordCategory::Supplier => b"supplier",
    }
}

// ======================================================
// EVENTOS
// ======================================================

#[event]
pub struct ProgramInitialized {
    pub admin: Pubkey,
}

#[event]
pub struct RecordCreated {
    pub id: u64,
    pub category: RecordCategory,
    pub contact_wallet: Pubkey,
    pub created_by: Pubkey,
}

#[event]
pub struct RecordUpdated {
    pub id: u64,
    pub category: RecordCategory,
}

#[event]
pub struct ActivityAdded {
    pub id: u64,
    pub category: RecordCategory,
    pub activity_count: u64,
}

#[event]
pub struct RecordDeactivated {
    pub id: u64,
    pub category: RecordCategory,
}

// ======================================================
// ERRORES PERSONALIZADOS
// ======================================================

#[error_code]
pub enum BellaAuraError {
    #[msg("El nombre no puede estar vacío.")]
    EmptyName,

    #[msg("El nombre supera el máximo de 64 caracteres.")]
    NameTooLong,

    #[msg("La nota de actividad no puede estar vacía.")]
    EmptyNote,

    #[msg("La nota supera el máximo de 120 caracteres.")]
    NoteTooLong,

    #[msg("El registro está inactivo.")]
    RecordInactive,

    #[msg("No se puede dar de baja: el registro tiene actividad.")]
    RecordHasActivity,

    #[msg("El contador superó el límite permitido.")]
    CounterOverflow,
}
