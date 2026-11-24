package fr.type1.keyboard

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.util.TypedValue
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewGroup
import android.widget.AbsListView
import android.widget.BaseAdapter
import android.widget.FrameLayout
import android.widget.GridView
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import kotlin.math.min

/**
 * The emoji panel opened from a long press on the comma. A category strip
 * (recent first) sits above a scrollable grid; the panel is allowed to grow
 * taller than the keyboard and is anchored just above it.
 */
internal class EmojiPickerPopup(
	private val context: Context,
	private val recentEmojis: () -> List<String>,
	private val onPick: (String) -> Unit,
	private val onDismissed: (EmojiPickerPopup) -> Unit,
) {

	private data class Tab(
		val icon: String,
		val description: String,
		val emojis: () -> List<String>,
	)

	private val tabs: List<Tab> = buildList {
		add(
			Tab(
				icon = RECENT_ICON,
				description = context.getString(R.string.emoji_recent),
				emojis = recentEmojis,
			),
		)
		for (category in EmojiData.categories) {
			add(
				Tab(
					icon = category.icon,
					description = context.getString(category.label),
					emojis = { category.emojis },
				),
			)
		}
	}

	private val accent = context.getColor(R.color.accent)
	private val selectedTabBackground = Color.argb(
		72,
		Color.red(accent),
		Color.green(accent),
		Color.blue(accent),
	)
	private val textColor = context.getColor(R.color.key_text)
	private val mutedColor = context.getColor(R.color.key_text_muted)
	private val panelBackground = context.getColor(R.color.keyboard_background)
	private val stripBackground = context.getColor(R.color.suggestion_background)

	private val adapter = EmojiAdapter()
	private val tabViews = ArrayList<TextView>()
	private var grid: GridView? = null
	private var emptyView: TextView? = null
	private var popup: PopupWindow? = null
	private var selectedTab = -1

	fun show(anchor: View): Boolean {
		if (popup?.isShowing == true || anchor.width <= 0) return false

		val margin = dp(4f)
		val widthPx = (anchor.width - margin * 2).coerceAtLeast(dp(200f)).toInt()
		// The space above the keyboard is measured on screen, but PopupWindow
		// positions a panel relative to its parent window.
		val onScreen = IntArray(2)
		anchor.getLocationOnScreen(onScreen)
		val above = onScreen[1] - dp(6f)
		val heightPx = (
			min(dp(PREFERRED_HEIGHT_DP), above)
			. coerceAtLeast(dp(MIN_HEIGHT_DP))
			. toInt()
		)

		val view = buildView(widthPx, heightPx)
		val window = PopupWindow(view, widthPx, heightPx, false).apply {
			isTouchable = true
			isFocusable = false
			isOutsideTouchable = true
			// A non-focusable popup only receives outside touches when it has a
			// background, so keep a transparent one.
			setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
			elevation = dp(16f)
			setOnDismissListener {
				popup = null
				onDismissed(this@EmojiPickerPopup)
			}
		}

		val location = IntArray(2)
		anchor.getLocationInWindow(location)
		val x = (location[0] + margin).toInt()
		val y = (location[1] - heightPx - margin).toInt()
		return try {
			window.showAtLocation(anchor, Gravity.NO_GRAVITY, x, y)
			popup = window
			true
		} catch (_: Exception) {
			false
		}
	}

	fun dismiss() {
		popup?.dismiss()
		popup = null
	}

	private fun buildView(widthPx: Int, heightPx: Int): View {
		val columns = (widthPx / dp(38f)).toInt().coerceIn(7, 9)
		val root = LinearLayout(context).apply {
			orientation = LinearLayout.VERTICAL
			background = GradientDrawable().apply {
				setColor(panelBackground)
				cornerRadius = dp(14f)
			}
			layoutParams = ViewGroup.LayoutParams(widthPx, heightPx)
		}

		root.addView(
			buildCategoryStrip(widthPx),
			LinearLayout.LayoutParams(
				ViewGroup.LayoutParams.MATCH_PARENT,
				dp(HEADER_HEIGHT_DP).toInt(),
			),
		)

		val empty = TextView(context).apply {
			text = context.getString(R.string.emoji_empty_recent)
			setTextColor(mutedColor)
			setTextSize(TypedValue.COMPLEX_UNIT_SP, 14f)
			gravity = Gravity.CENTER
			visibility = View.GONE
		}
		emptyView = empty

		val emojiGrid = GridView(context).apply {
			numColumns = columns
			stretchMode = GridView.STRETCH_COLUMN_WIDTH
			horizontalSpacing = 0
			verticalSpacing = dp(2f).toInt()
			isVerticalScrollBarEnabled = false
			setPadding(dp(4f).toInt(), dp(2f).toInt(), dp(4f).toInt(), dp(6f).toInt())
			clipToPadding = false
			adapter = this@EmojiPickerPopup.adapter
			setOnItemClickListener { _, _, position, _ ->
				pick(position)
			}
		}
		emojiGrid.emptyView = empty
		grid = emojiGrid

		root.addView(
			FrameLayout(context).apply {
				addView(
					emojiGrid,
					FrameLayout.LayoutParams(
						ViewGroup.LayoutParams.MATCH_PARENT,
						ViewGroup.LayoutParams.MATCH_PARENT,
					),
				)
				addView(
					empty,
					FrameLayout.LayoutParams(
						ViewGroup.LayoutParams.MATCH_PARENT,
						ViewGroup.LayoutParams.MATCH_PARENT,
					),
				)
			},
			LinearLayout.LayoutParams(
				ViewGroup.LayoutParams.MATCH_PARENT,
				0,
				1f,
			),
		)

		select(0, scrollToTop = true)
		return root
	}

	private fun buildCategoryStrip(availableWidth: Int): View {
		// Fit every category when the keyboard is wide enough, like the strip in
		// the reference panel; only very narrow screens need to scroll.
		val padding = dp(8f).toInt()
		val margins = (dp(2f) * tabs.size).toInt()
		val tabSize = min(
			dp(TAB_MAX_DP),
			(availableWidth - padding - margins).toFloat() / tabs.size,
		).coerceAtLeast(dp(TAB_MIN_DP)).toInt()
		val row = LinearLayout(context).apply {
			orientation = LinearLayout.HORIZONTAL
			gravity = Gravity.CENTER_VERTICAL
			setPadding(dp(4f).toInt(), dp(4f).toInt(), dp(4f).toInt(), dp(4f).toInt())
		}
		for (index in tabs.indices) {
			val tab = tabs[index]
			val item = TextView(context).apply {
				text = tab.icon
				contentDescription = tab.description
				gravity = Gravity.CENTER
				setTextSize(
					TypedValue.COMPLEX_UNIT_SP,
					if (tab.icon.length > 2) MULTI_CHAR_TAB_SP else TAB_SP,
				)
				setTextColor(textColor)
				setTypeface(Typeface.create("sans-serif", Typeface.NORMAL))
				isClickable = true
				isFocusable = true
				setOnClickListener {
					performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
					select(index, scrollToTop = true)
				}
			}
			row.addView(
				item,
				LinearLayout.LayoutParams(tabSize, dp(TAB_SIZE_DP).toInt()).apply {
					marginEnd = dp(2f).toInt()
				},
			)
			tabViews += item
		}

		val radius = dp(14f)
		val scroll = HorizontalScrollView(context).apply {
			isHorizontalScrollBarEnabled = false
			overScrollMode = View.OVER_SCROLL_NEVER
			background = GradientDrawable().apply {
				setColor(stripBackground)
				cornerRadii = floatArrayOf(radius, radius, radius, radius, 0f, 0f, 0f, 0f)
			}
			addView(
				row,
				ViewGroup.LayoutParams(
					ViewGroup.LayoutParams.WRAP_CONTENT,
					ViewGroup.LayoutParams.MATCH_PARENT,
				),
			)
		}
		return scroll
	}

	private fun select(index: Int, scrollToTop: Boolean) {
		if (index !in tabs.indices) return
		selectedTab = index
		for (i in tabViews.indices) {
			val selected = i == index
			tabViews[i].background = if (selected) {
				GradientDrawable().apply {
					setColor(selectedTabBackground)
					cornerRadius = dp(9f)
				}
			} else {
				null
			}
			tabViews[i].setTextColor(if (selected) textColor else mutedColor)
		}
		adapter.submit(tabs[index].emojis())
		if (scrollToTop) grid?.setSelection(0)
		emptyView?.visibility = if (adapter.count == 0) View.VISIBLE else View.GONE
	}

	private fun pick(position: Int) {
		val emoji = adapter.items.getOrNull(position) ?: return
		onPick(emoji)
		// The recent list changed; keep the current category stable but refresh
		// the recent tab when the user is looking at it.
		if (selectedTab == 0) select(0, scrollToTop = true)
	}

	private fun dp(value: Float): Float = value * context.resources.displayMetrics.density

	private inner class EmojiAdapter : BaseAdapter() {

		var items: List<String> = emptyList()
			private set

		fun submit(next: List<String>) {
			items = next
			notifyDataSetChanged()
		}

		override fun getCount(): Int = items.size

		override fun getItem(position: Int): String = items[position]

		override fun getItemId(position: Int): Long = position.toLong()

		override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
			val label = convertView as? TextView ?: TextView(context).apply {
				gravity = Gravity.CENTER
				isSingleLine = true
				includeFontPadding = false
				setTextColor(textColor)
				layoutParams = AbsListView.LayoutParams(
					ViewGroup.LayoutParams.MATCH_PARENT,
					dp(CELL_HEIGHT_DP).toInt(),
				)
			}
			val text = items[position]
			label.text = text
			// Kaomoji are much wider than a single emoji; shrink them so they
			// stay inside their cell.
			val size = when {
				text.length <= 4 -> EMOJI_TEXT_SP
				text.length <= 7 -> 15f
				else -> 10f
			}
			label.setTextSize(TypedValue.COMPLEX_UNIT_SP, size)
			return label
		}
	}

	private companion object {
		const val RECENT_ICON = "🕘"
		const val HEADER_HEIGHT_DP = 50f
		const val TAB_MAX_DP = 40f
		const val TAB_MIN_DP = 26f
		const val TAB_SIZE_DP = 38f
		const val TAB_SP = 17f
		const val MULTI_CHAR_TAB_SP = 13f
		const val CELL_HEIGHT_DP = 44f
		const val EMOJI_TEXT_SP = 23f
		const val PREFERRED_HEIGHT_DP = 320f
		const val MIN_HEIGHT_DP = 180f
	}
}
