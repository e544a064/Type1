//! Tiny embedded French lexicon used for prefix suggestions.
//!
//! This is intentionally small: enough to make the suggestion strip useful and
//! to prove the pipeline. Replacing it with a frequency-ordered dictionary (or
//! a real prediction model) does not change the engine API.

/// Accent-insensitive fold so `ecole` can suggest `école`.
pub fn fold(input: &str) -> String {
	let mut out = String::with_capacity(input.len());
	for c in input.chars() {
		for lower in c.to_lowercase() {
			match lower {
				'œ' => out.push_str("oe"),
				'æ' => out.push_str("ae"),
				'à' | 'â' | 'ä' | 'á' => out.push('a'),
				'é' | 'è' | 'ê' | 'ë' => out.push('e'),
				'î' | 'ï' | 'í' => out.push('i'),
				'ô' | 'ö' | 'ó' => out.push('o'),
				'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
				'ç' => out.push('c'),
				'ÿ' => out.push('y'),
				'ñ' => out.push('n'),
				other => out.push(other),
			}
		}
	}
	out
}

/// Words whose folded form starts with the folded prefix, in list order.
/// The typed prefix itself is never returned.
pub fn suggest(prefix: &str, limit: usize) -> Vec<&'static str> {
	if prefix.chars().count() < 2 {
		return Vec::new();
	}
	let folded = fold(prefix);
	let mut out = Vec::with_capacity(limit);
	for word in WORDS {
		if out.len() >= limit {
			break;
		}
		let candidate = fold(word);
		if candidate.len() > folded.len() && candidate.starts_with(&folded) {
			out.push(*word);
		}
	}
	out
}

/// Common French words, roughly frequency ordered within each group.
pub static WORDS: &[&str] = &[
	// function words
	"le", "la", "les", "un", "une", "des", "de", "du", "et", "est", "en", "que", "qui",
	"quoi", "dont", "où", "quand", "comment", "pourquoi", "combien", "quel", "quelle",
	"quels", "quelles", "je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles",
	"me", "te", "se", "moi", "toi", "lui", "leur", "eux", "mon", "ma", "mes", "ton", "ta",
	"tes", "son", "sa", "ses", "notre", "nos", "votre", "vos", "leurs", "ce", "cet",
	"cette", "ces", "celui", "celle", "ceux", "celles", "à", "au", "aux", "avec", "sans",
	"sous", "sur", "dans", "par", "pour", "vers", "chez", "entre", "contre", "depuis",
	"pendant", "avant", "après", "malgré", "ne", "pas", "plus", "moins", "très", "trop",
	"assez", "bien", "mal", "mieux", "tout", "tous", "toute", "toutes", "rien", "personne",
	"jamais", "toujours", "souvent", "parfois", "déjà", "encore", "aussi", "alors", "donc",
	"mais", "ou", "or", "ni", "car", "si", "comme", "lorsque", "puisque", "tandis",
	// verbs
	"être", "avoir", "faire", "dire", "aller", "venir", "voir", "savoir", "pouvoir",
	"vouloir", "devoir", "falloir", "prendre", "mettre", "donner", "parler", "penser",
	"trouver", "passer", "rester", "tenir", "porter", "montrer", "demander", "répondre",
	"écouter", "entendre", "regarder", "lire", "écrire", "manger", "boire", "dormir",
	"partir", "arriver", "rentrer", "sortir", "monter", "descendre", "ouvrir", "fermer",
	"acheter", "vendre", "payer", "travailler", "jouer", "aimer", "préférer", "choisir",
	"commencer", "finir", "attendre", "comprendre", "apprendre", "connaître", "vivre",
	"mourir", "devenir", "sembler", "paraître", "croire", "espérer", "oublier", "rappeler",
	"essayer", "aider", "laisser", "garder", "changer", "créer", "utiliser", "gagner",
	"perdre", "tomber", "lever", "courir", "marcher", "voyager", "envoyer", "recevoir",
	"ouvrir", "remplir", "suivre", "produire", "construire", "décider", "expliquer",
	// everyday
	"oui", "non", "peut-être", "merci", "bonjour", "salut", "revoir", "bienvenue",
	"pardon", "excusez", "plaît", "temps", "jour", "jours", "nuit", "matin", "soir",
	"semaine", "mois", "année", "an", "heure", "minute", "seconde", "moment", "fois",
	"instant", "maison", "appartement", "travail", "bureau", "école", "université",
	"enfant", "enfants", "femme", "homme", "ami", "amie", "amis", "famille", "père",
	"mère", "frère", "sœur", "fils", "fille", "monde", "vie", "main", "mains", "tête",
	"yeux", "cœur", "corps", "visage", "voix", "nom", "prénom", "âge", "chose", "choses",
	"point", "partie", "endroit", "place", "ville", "pays", "région", "rue", "route",
	"chemin", "porte", "fenêtre", "table", "chaise", "lit", "cuisine", "salle", "jardin",
	"voiture", "train", "bus", "avion", "métro", "vélo", "eau", "pain", "café", "thé",
	"lait", "sucre", "sel", "riz", "viande", "poisson", "légume", "fruit", "pomme",
	"fromage", "vin", "bière", "repas", "déjeuner", "dîner", "argent", "prix", "euro",
	"euros", "emploi", "entreprise", "client", "projet", "idée", "problème", "question",
	"réponse", "raison", "exemple", "façon", "manière", "sorte", "genre", "type",
	"groupe", "équipe", "réunion", "message", "téléphone", "ordinateur", "internet",
	"site", "page", "fichier", "photo", "vidéo", "musique", "film", "livre", "journal",
	"article", "mot", "phrase", "texte", "langue", "français", "anglais",
	// adjectives
	"bon", "bonne", "mauvais", "grand", "grande", "petit", "petite", "jeune", "vieux",
	"vieille", "nouveau", "nouvelle", "beau", "belle", "joli", "jolie", "long", "longue",
	"court", "courte", "haut", "haute", "bas", "basse", "gros", "grosse", "fort",
	"faible", "facile", "difficile", "important", "nécessaire", "possible", "impossible",
	"vrai", "faux", "même", "autre", "premier", "dernier", "prochain", "seul", "propre",
	"libre", "plein", "vide", "chaud", "froid", "meilleur", "pire",
	// time and place
	"aujourd'hui", "hier", "demain", "maintenant", "bientôt", "tard", "tôt", "ensuite",
	"enfin", "d'abord", "ici", "là", "ailleurs", "partout", "dedans", "dehors",
	"dessus", "dessous", "devant", "derrière", "près", "loin", "autour", "ensemble",
	"vite", "lentement", "beaucoup", "peu", "environ", "presque", "seulement", "surtout",
	"vraiment", "sûrement", "certainement", "probablement", "parce", "ainsi",
	"cependant", "pourtant", "néanmoins", "toutefois", "sinon", "autrement", "notamment",
	"également", "voilà", "voici",
];

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn folds_accents() {
		assert_eq!(fold("École"), "ecole");
		assert_eq!(fold("où"), "ou");
		assert_eq!(fold("œuf"), "oeuf");
	}

	#[test]
	fn suggests_accented_words_from_plain_prefix() {
		let found = suggest("ec", 5);
		assert!(found.contains(&"école"), "got {found:?}");
	}

	#[test]
	fn needs_two_characters() {
		assert!(suggest("e", 5).is_empty());
	}

	#[test]
	fn excludes_the_prefix_itself() {
		assert!(!suggest("temps", 5).contains(&"temps"));
	}
}
