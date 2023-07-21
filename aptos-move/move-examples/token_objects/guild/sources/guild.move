/// This module implements the guild token (non-fungible token) including the
/// functions create the collection and the guild tokens.
module token_objects::guild {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_framework::object::{Self, Object};
    use aptos_token_objects::collection;
    use aptos_token_objects::token;

    /// The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The provided signer is not the admin
    const ENOT_ADMIN: u64 = 2;
    /// The provided signer is not the owner
    const ENOT_OWNER: u64 = 3;
    /// The provided signer is not a guild master
    const ENOT_GUILD_MASTER: u64 = 4;

    /// The guild token collection name
    const GUILD_COLLECTION_NAME: vector<u8> = b"Guild Collection Name";
    /// The guild collection description
    const GUILD_COLLECTION_DESCRIPTION: vector<u8> = b"Guild Collection Description";
    /// The guild collection URI
    const GUILD_COLLECTION_URI: vector<u8> = b"https://guild.collection.uri";

    /// Whitelisted guild masters
    struct Config has key {
        whitelist: vector<address>,
    }

    /// Manager object refs
    struct Manager has key {
        /// The extend_ref of the manager object to get its signer
        extend_ref: object::ExtendRef,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Guild token
    struct GuildToken has key {
        /// Used to get the signer of the token
        extend_ref: object::ExtendRef,
        /// Member collection name
        member_collection: String,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Member token
    struct MemberToken has key {
        /// Belonging guild
        guild: Object<GuildToken>,
        /// Used to burn
        burn_ref: token::BurnRef,
    }

    /// Initializes the module, creating the manager object, the guild token collection and the whitelist.
    fun init_module(sender: &signer) acquires Manager {
        // Create the manager object to manage the guild collection.
        create_manager(sender);
        // Create a collection for guild tokens.
        create_guild_collection(&manager_signer());
        // Create a whitelist
        move_to(sender, Config { whitelist: vector::empty() });
    }

    #[view]
    /// Returns the guild token address by name
    public fun guild_token_address(guild_token_name: String): address acquires Manager {
        token::create_token_address(&manager_address(), &string::utf8(GUILD_COLLECTION_NAME), &guild_token_name)
    }

    #[view]
    /// Returns the guild token address by name
    public fun member_token_address(guild_token: Object<GuildToken>, member_token_name: String): address acquires GuildToken {
        let guild_token_addr = object::object_address(&guild_token);
        let member_collection_name = &borrow_global<GuildToken>(copy guild_token_addr).member_collection;
        token::create_token_address(&guild_token_addr, member_collection_name, &member_token_name)
    }

    /// Adds a guild master to the whitelist. This function allows the admin to add a guild master
    /// to the whitelist.
    public entry fun add_guild_master(admin: &signer, guild_master: address) acquires Config {
        assert!(signer::address_of(admin) == @token_objects, ENOT_ADMIN);
        let config = borrow_global_mut<Config>(@token_objects);
        vector::push_back(&mut config.whitelist, guild_master);
    }

    /// Mints an guild token, and creates a new associated member collection.
    /// This function allows a whitelisted guild master to mint a new guild token.
    public entry fun mint_guild(
        guild_master: &signer,
        description: String,
        name: String,
        uri: String,
        member_collection_name: String,
        member_collection_description: String,
        member_collection_uri: String,
    ) acquires Config, Manager {
        // Checks if the guild master is whitelisted.
        let guild_master_addr = signer::address_of(guild_master);
        assert!(is_whitelisted(guild_master_addr), ENOT_GUILD_MASTER);

        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(GUILD_COLLECTION_NAME);
        // Creates the guild token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let constructor_ref = token::create_named_token(
            &manager_signer(),
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, guild_master_addr);

        // Publishes the GuildToken resource with the refs.
        let guild_token = GuildToken {
            extend_ref,
            member_collection: member_collection_name,
        };
        move_to(&object_signer, guild_token);

        // Creates a member collection which is associated to the guild token.
        create_member_collection(&object_signer, member_collection_name, member_collection_description, member_collection_uri);
    }

    /// Mints an member token. This function mints a new member token and transfers it to the
    /// `receiver` address.
    public entry fun mint_member(
        guild_master: &signer,
        guild_token: Object<GuildToken>,
        description: String,
        name: String,
        uri: String,
        receiver: address,
    ) acquires GuildToken {
        // Checks if the guild master is the owner of the guild token.
        assert!(object::owner(guild_token) == signer::address_of(guild_master), ENOT_OWNER);

        let guild = borrow_global<GuildToken>(object::object_address(&guild_token));
        let guild_token_object_signer = object::generate_signer_for_extending(&guild.extend_ref);
        // Creates the member token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let constructor_ref = token::create_named_token(
            &guild_token_object_signer,
            guild.member_collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let burn_ref = token::generate_burn_ref(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, receiver);

        // Publishes the MemberToken resource with the refs.
        let member_token = MemberToken {
            guild: guild_token,
            burn_ref,
        };
        move_to(&object_signer, member_token);
    }

    /// Burns an member token.
    public entry fun burn_member(
        guild_master: &signer,
        token: Object<MemberToken>,
    ) acquires MemberToken {
        let belonging_guild = borrow_global<MemberToken>(object::object_address(&token)).guild;
        assert!(object::owner(belonging_guild) == signer::address_of(guild_master), ENOT_OWNER);
        let member_token = move_from<MemberToken>(object::object_address(&token));
        let MemberToken {
            guild: _,
            burn_ref,
        } = member_token;
        token::burn(burn_ref);
    }

    /// Creates the manager object.
    fun create_manager(sender: &signer) {
        let constructor_ref = object::create_object(signer::address_of(sender));
        let extend_ref = object::generate_extend_ref(&constructor_ref);
        move_to(sender, Manager { extend_ref });
    }

    /// Returns the signer of the manager object.
    fun manager_signer(): signer acquires Manager {
        let manager = borrow_global<Manager>(@token_objects);
        object::generate_signer_for_extending(&manager.extend_ref)
    }

    /// Returns the address of the manager object.
    fun manager_address(): address acquires Manager {
        let manager = borrow_global<Manager>(@token_objects);
        object::address_from_extend_ref(&manager.extend_ref)
    }

    /// Creates the guild collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_guild_collection(admin: &signer) {
        // Constructs the strings from the bytes.
        let description = string::utf8(GUILD_COLLECTION_DESCRIPTION);
        let name = string::utf8(GUILD_COLLECTION_NAME);
        let uri = string::utf8(GUILD_COLLECTION_URI);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            admin,
            description,
            name,
            option::none(),
            uri,
        );
    }

    /// Creates the member collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_member_collection(guild_token_object_signer: &signer, name: String, description: String, uri: String) {
        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            guild_token_object_signer,
            description,
            name,
            option::none(),
            uri,
        );
    }

    inline fun is_whitelisted(guild_master: address): bool {
        let whitelist = &borrow_global<Config>(@token_objects).whitelist;
        vector::contains(whitelist, &guild_master)
    }

    #[test(fx = @std, admin = @token_objects, guild_master = @0x456, user = @0x789)]
    public fun test_guild(fx: signer, admin: &signer, guild_master: &signer, user: address) acquires GuildToken, MemberToken, Manager, Config {
        use std::features;

        let feature = features::get_auids();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        // This test assumes that the creator's address is equal to @token_objects.
        assert!(signer::address_of(admin) == @token_objects, 0);

        // -----------------------------------
        // Admin creates the guild collection.
        // -----------------------------------
        init_module(admin);

        // ---------------------------------------------
        // Admin adds the guild master to the whitelist.
        // ---------------------------------------------
        add_guild_master(admin, signer::address_of(guild_master));

        // ------------------------------------------
        // Guild master mints a guild token.
        // ------------------------------------------
        mint_guild(
            guild_master,
            string::utf8(b"Guild Token #1 Description"),
            string::utf8(b"Guild Token #1"),
            string::utf8(b"Guild Token #1 URI"),
            string::utf8(b"Member Collection #1"),
            string::utf8(b"Member Collection #1 Description"),
            string::utf8(b"Member Collection #1 URI"),
        );

        // -------------------------------------------
        // Guild master mints a member token for User.
        // -------------------------------------------
        let token_name = string::utf8(b"Member Token #1");
        let token_description = string::utf8(b"Member Token #1 Description");
        let token_uri = string::utf8(b"Member Token #1 URI");
        let guild_token_addr = guild_token_address(string::utf8(b"Guild Token #1"));
        let guild_token = object::address_to_object<GuildToken>(guild_token_addr);
        // Creates the member token for User.
        mint_member(
            guild_master,
            guild_token,
            token_description,
            token_name,
            token_uri,
            user,
        );

        // ------------------------------------------------
        // Guild master burns the member token of the User.
        // ------------------------------------------------
        let member_token_addr = member_token_address(guild_token, token_name);
        burn_member(guild_master, object::address_to_object<MemberToken>(member_token_addr));
    }
}
