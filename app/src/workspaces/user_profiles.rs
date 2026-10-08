use std::collections::HashMap;

use warpui::{Entity, SingletonEntity};

use crate::auth::UserUid;

/// A user profile, as stored in sqlite or received from the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserProfileWithUID {
    pub firebase_uid: UserUid,
    pub display_name: Option<String>,
    pub email: String,
    pub photo_url: String,
}

impl From<warp_graphql::user::PublicUserProfile> for UserProfileWithUID {
    fn from(value: warp_graphql::user::PublicUserProfile) -> Self {
        UserProfileWithUID {
            firebase_uid: UserUid::new(&value.uid),
            display_name: value.display_name,
            email: value.email.unwrap_or_default(),
            photo_url: value.photo_url.unwrap_or_default(),
        }
    }
}

pub enum UserProfilesEvent {}

#[cfg(not(target_family = "wasm"))]
pub fn user_profile_from_persistence(
    user_profile: crate::persistence::model::UserProfile,
) -> UserProfileWithUID {
    UserProfileWithUID {
        firebase_uid: UserUid::new(&user_profile.firebase_uid),
        display_name: user_profile.display_name,
        email: user_profile.email,
        photo_url: user_profile.photo_url,
    }
}

/// Private struct for internal mapping between the user's uid and the important information we might
/// want to query about them.
pub struct UserProfileData {
    pub display_name: Option<String>,
    pub email: String,
    #[allow(dead_code)]
    pub photo_url: String,
}

/// UserProfiles is a singleton model storing data on adjacent users (e.g., teammates or former teammates). The
/// purpose of this model is to quickly convert the UID for some user into displayable information about them;
/// for example, their name, email, or  profile photo. This allows us to display a richer view into the history
/// of objects and the users who have created, executed, or edited them, etc.
pub struct UserProfiles {
    users_by_id: HashMap<UserUid, UserProfileData>,
}

impl UserProfiles {
    pub fn new(user_profiles: Vec<UserProfileWithUID>) -> Self {
        let mut model = Self {
            users_by_id: HashMap::new(),
        };

        model.insert_profiles(&user_profiles);

        model
    }

    /// Accepts a vector of user profiles and inserts them into the model, overwriting
    /// the old version of a profile if it already exists.
    pub fn insert_profiles(&mut self, user_profiles: &Vec<UserProfileWithUID>) {
        for user_profile in user_profiles {
            self.users_by_id.insert(
                user_profile.firebase_uid,
                UserProfileData {
                    display_name: user_profile.display_name.clone(),
                    email: user_profile.email.clone(),
                    photo_url: user_profile.photo_url.clone(),
                },
            );
        }
    }

}

impl UserProfileData {
}

impl Entity for UserProfiles {
    type Event = UserProfilesEvent;
}

impl SingletonEntity for UserProfiles {}
